use std::cmp::max;
use std::sync::Arc;

use anyhow::Context as _;
use azookey_server::TonicNamedPipeServer;
use ipc::{WindowAction, WindowController, WindowService};
use shared::proto::window_service_server::WindowServiceServer;
use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::platform::windows::{EventLoopBuilderExtWindows, WindowExtWindows};
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tonic::transport::Server;
use uiaccess::prepare_uiaccess_token;
use utils::get_candidate_window_position;
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE,
};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE},
};

pub mod candidate;
pub mod indicator;
pub mod ipc;
pub mod uiaccess;
pub mod utils;

#[derive(Debug)]
pub enum UserEvent {
    UpdateCandidates(String),
    UpdateSelection(i32),
    UpdateInputMethod(String),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // obtain uiaccess token
    prepare_uiaccess_token()?;

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event()
        .with_any_thread(true)
        .build();

    let candidate_window = candidate::create_candidate_window(&event_loop)?;
    let candidate_webview = candidate::create_candidate_webview(&candidate_window)?;

    let indicator_window = indicator::create_indicator_window(&event_loop)?;
    let indicator_webview = indicator::create_indicator_webview(&indicator_window)?;

    // initialize window controller
    let (tx, mut rx) = mpsc::channel(32);
    let window_controller = WindowController::new(tx.clone());
    let grpc_service = WindowService {
        controller: window_controller.clone(),
    };

    // start grpc server
    tokio::spawn(async move {
        println!("WindowServer listening");
        Server::builder()
            .add_service(WindowServiceServer::new(grpc_service))
            .serve_with_incoming(TonicNamedPipeServer::new("azookey_ui"))
            .await
            .expect("gRPC server failed");
    });

    let event_loop_proxy = event_loop.create_proxy();
    let task_guard: Arc<Mutex<Option<JoinHandle<()>>>> = Arc::new(Mutex::new(None));

    // handle window actions
    tokio::spawn(async move {
        while let Some(action) = rx.recv().await {
            let indicator_hwnd = indicator_window.hwnd();

            match action {
                WindowAction::Show => {
                    // if mode indicator is already shown, hide it
                    let mut task_guard = task_guard.lock().await;
                    if let Some(task) = task_guard.take() {
                        task.abort();
                        let _ = unsafe {
                            ShowWindow(HWND(indicator_hwnd as *mut std::ffi::c_void), SW_HIDE)
                        };
                    }

                    let _ = unsafe {
                        ShowWindow(
                            HWND(candidate_window.hwnd() as *mut std::ffi::c_void),
                            SW_SHOWNOACTIVATE,
                        )
                    };
                }
                WindowAction::Hide => {
                    let _ = unsafe {
                        ShowWindow(
                            HWND(candidate_window.hwnd() as *mut std::ffi::c_void),
                            SW_HIDE,
                        )
                    };
                }
                WindowAction::SetPosition {
                    top,
                    left,
                    bottom,
                    right,
                } => {
                    let (x, y) =
                        get_candidate_window_position(top, left, bottom, right, &candidate_window);

                    unsafe {
                        let _ = SetWindowPos(
                            HWND(candidate_window.hwnd() as *mut std::ffi::c_void),
                            HWND_TOPMOST,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        );

                        let _ = SetWindowPos(
                            HWND(indicator_hwnd as *mut std::ffi::c_void),
                            HWND_TOPMOST,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                    candidate_window.set_outer_position(PhysicalPosition::new(x as f64, y as f64));
                    indicator_window.set_outer_position(PhysicalPosition::new(
                        (left - 45) as f64,
                        bottom as f64,
                    ));
                }
                WindowAction::SetCandidate { candidates } => {
                    let max_len = candidates
                        .iter()
                        .map(|s| s.chars().count())
                        .max()
                        .unwrap_or(0) as u32;
                    candidate_window
                        .set_inner_size(PhysicalSize::new(max(225, 120 + max_len * 18), 275));

                    let candidates = serde_json::to_string(&candidates)
                        .context("Failed to serialize candidates")
                        .unwrap();

                    event_loop_proxy
                        .send_event(UserEvent::UpdateCandidates(candidates))
                        .unwrap();
                }
                WindowAction::SetSelection { index } => {
                    event_loop_proxy
                        .send_event(UserEvent::UpdateSelection(index))
                        .unwrap();
                }
                WindowAction::SetInputMode(input_method) => {
                    event_loop_proxy
                        .send_event(UserEvent::UpdateInputMethod(input_method))
                        .unwrap();

                    let mut task_guard = task_guard.lock().await;
                    if let Some(task) = task_guard.take() {
                        task.abort();
                    }

                    *task_guard = Some(tokio::spawn(async move {
                        let _ = unsafe {
                            ShowWindow(
                                HWND(indicator_hwnd as *mut std::ffi::c_void),
                                SW_SHOWNOACTIVATE,
                            )
                        };
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        let _ = unsafe {
                            ShowWindow(HWND(indicator_hwnd as *mut std::ffi::c_void), SW_HIDE)
                        };
                    }));
                }
            }
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {}
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(script) => match script {
                UserEvent::UpdateCandidates(candidates) => {
                    candidate_webview
                        .evaluate_script(&format!("updateCandidates({})", candidates))
                        .unwrap();
                }
                UserEvent::UpdateSelection(index) => {
                    candidate_webview
                        .evaluate_script(&format!("updateSelection({})", index))
                        .unwrap();
                }
                UserEvent::UpdateInputMethod(input_method) => {
                    indicator_webview
                        .evaluate_script(&format!("updateInputMethod(\"{}\")", input_method))
                        .unwrap();
                }
            },
            _ => (),
        }
    });
}
