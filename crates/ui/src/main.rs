use std::cmp::max;

use anyhow::Context as _;
use azookey_server::TonicNamedPipeServer;
use protos::proto::window_service_server::{
    WindowService as WindowServiceProto, WindowServiceServer,
};
use protos::proto::{EmptyResponse, SetCandidateRequest, SetPositionRequest, SetSelectionRequest};
use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::platform::windows::{
    EventLoopBuilderExtWindows, WindowBuilderExtWindows, WindowExtWindows,
};
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use tokio::sync::mpsc;
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use uiaccess::prepare_uiaccess_token;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromRect, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE,
};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        SetWindowLongW, ShowWindow, GWL_EXSTYLE, GWL_STYLE, SW_SHOWNOACTIVATE, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    },
};
use wry::WebViewBuilder;

pub mod uiaccess;

#[derive(Debug, Clone)]
struct WindowController {
    sender: mpsc::Sender<WindowAction>,
}

impl WindowController {
    fn new(sender: mpsc::Sender<WindowAction>) -> Self {
        Self { sender }
    }
}

// ウィンドウ操作コマンド
#[derive(Debug, serde::Serialize)]
enum WindowAction {
    Show,
    Hide,
    SetPosition {
        top: i32,
        left: i32,
        bottom: i32,
        right: i32,
    },
    SetSelection {
        index: i32,
    },
    SetCandidate {
        candidates: Vec<String>,
    },
}

#[derive(Debug)]
struct WindowService {
    controller: WindowController,
}

#[tonic::async_trait]
impl WindowServiceProto for WindowService {
    async fn show_window(
        &self,
        _request: Request<EmptyResponse>,
    ) -> Result<Response<EmptyResponse>, Status> {
        self.controller
            .sender
            .send(WindowAction::Show)
            .await
            .unwrap();
        Ok(Response::new(EmptyResponse {}))
    }

    async fn hide_window(
        &self,
        _request: Request<EmptyResponse>,
    ) -> Result<Response<EmptyResponse>, Status> {
        self.controller
            .sender
            .send(WindowAction::Hide)
            .await
            .unwrap();
        Ok(Response::new(EmptyResponse {}))
    }
    async fn set_window_position(
        &self,
        request: Request<SetPositionRequest>,
    ) -> Result<Response<EmptyResponse>, Status> {
        let position = request.into_inner().position.unwrap();
        let top = position.top;
        let left = position.left;
        let bottom = position.bottom;
        let right = position.right;
        self.controller
            .sender
            .send(WindowAction::SetPosition {
                top,
                left,
                bottom,
                right,
            })
            .await
            .unwrap();

        Ok(Response::new(EmptyResponse {}))
    }

    async fn set_candidate(
        &self,
        request: Request<SetCandidateRequest>,
    ) -> Result<Response<EmptyResponse>, Status> {
        let candidate = request.into_inner().candidates;

        self.controller
            .sender
            .send(WindowAction::SetCandidate {
                candidates: candidate,
            })
            .await
            .unwrap();

        Ok(Response::new(EmptyResponse {}))
    }

    async fn set_selection(
        &self,
        request: Request<SetSelectionRequest>,
    ) -> Result<Response<EmptyResponse>, Status> {
        let index = request.into_inner().index;
        self.controller
            .sender
            .send(WindowAction::SetSelection { index })
            .await
            .unwrap();

        Ok(Response::new(EmptyResponse {}))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // obtain uiaccess token
    prepare_uiaccess_token()?;

    let event_loop = EventLoopBuilder::<String>::with_user_event()
        .with_any_thread(true)
        .build();
    let window = WindowBuilder::new()
        .with_decorations(false)
        .with_title("CandidateList")
        .with_focused(false)
        .with_visible(false)
        .with_undecorated_shadow(false)
        .with_transparent(true)
        .build(&event_loop)
        .context("Failed to create window")?;

    // set size
    window.set_inner_size(PhysicalSize::new(275.0, 275.0));

    let hwnd = window.hwnd() as *mut std::ffi::c_void;

    // set extended window style
    // https://docs.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles
    // https://docs.microsoft.com/en-us/windows/win32/winmsg/window-styles
    unsafe {
        let exnewstyle = WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0 | WS_EX_TOPMOST.0;
        SetWindowLongW(HWND(hwnd), GWL_EXSTYLE, exnewstyle as i32);

        let style = WS_POPUP.0;
        SetWindowLongW(HWND(hwnd), GWL_STYLE, style as i32);
    };

    let webview = WebViewBuilder::new()
        .with_transparent(true)
        .with_html(
            r##"
            <html>
                <head>
                    <style>
                        body, html {
                            overscroll-behavior: none;
                        }
                        body {
                            margin: 0;
                            padding: 7px;
                            filter: drop-shadow(3px 3px 3px rgba(0, 0, 0, 0.1));
                        }
                        main {
                            width: 100%;
                            height: 100%;
                            padding: 8 8 30 8;
                            border: 1px solid #E4E4E4;
                            border-radius: 10px;
                            background-color: #FFFFFF;
                            box-sizing: border-box;
                        }
                        ol {
                            margin: 0;
                            padding: 0;
                            height: 100%;
                            overflow-y: scroll;
                            scroll-snap-type: y proximity;
                            list-style-position: inside;
                            list-style-type: none;
                            counter-reset: number 0;
                            user-select: none;
                            cursor: pointer;

                            &::-webkit-scrollbar {
                                width: 5px;
                            }

                            &::-webkit-scrollbar-thumb {
                                background-color: #BCBCBC;
                                border-radius: 10px;
                            }
                        }
                        li {
                            padding: 0.5rem;
                            font-size: 0.9rem;
                            display: flex;
                            align-items: center;
                            scroll-snap-align: start;

                            &::before {
                                content: counter(number);
                                counter-increment: number 1;
                                color: #636363;
                                font-weight: bold;
                                font-size: 0.75rem;
                                margin: 0 0.75rem 0 2;
                            }

                            &[data-selected] {
                                background-color: #D4F0FF;
                                border-radius: 3px;
                                margin-right: 5px;
                                outline: 1px solid #2CB5FF;
                                outline-offset: -1px;
                            }
                        }
                        footer {
                            display: flex;
                            justify-content: space-between;
                            align-items: center;
                            padding: 8 10 5 10;
                            border-top: 1px solid #E4E4E4;
                            font-size: 0.8rem;
                            user-select: none;
                        }

                        @media (prefers-color-scheme: dark) {
                            body {
                                color: #FFFFFF;
                            }
                            main {
                                border: 1px solid #424242;
                                background-color: #1E1E1E;
                            }
                            ol::-webkit-scrollbar-thumb {
                                background-color: #757575;
                            }
                            li {
                                color: #E0E0E0;
                            
                                &::before {
                                    color: #BDBDBD;
                                }

                                &[data-selected] {
                                    background-color: #3949AB;
                                    outline: 1px solid #5C6BC0;
                                }
                            }
                                
                            footer {
                                border-top: 1px solid #424242;
                            }
                        }
                    </style>
                    <script>
                        function updateCandidates(candidates) {
                            const candidateList = document.getElementById('candidate-list');

                            const existingItems = Array.from(candidateList.children);

                            candidates.forEach((candidate, index) => {
                                if (existingItems[index]) {
                                    existingItems[index].textContent = candidate;
                                } else {
                                    const li = document.createElement('li');
                                    li.textContent = candidate;
                                    candidateList.appendChild(li);
                                }
                            });

                            while (existingItems.length > candidates.length) {
                                candidateList.removeChild(existingItems.pop());
                            }
                        }

                        function updateSelection(index) {
                            const candidateList = document.getElementById('candidate-list');
                            const selected = candidateList.querySelector('[data-selected]');
                            if (selected) {
                                selected.removeAttribute('data-selected');
                            }
                            candidateList.children[index].setAttribute('data-selected', '');
                            candidateList.children[index].scrollIntoView({ behavior: "instant", block: "start", inline: "start" });
                        }
                    </script>
                </head>
                <body style="margin: 0;">
                    <main>
                        <ol id="candidate-list">
                        </ol>
                        <footer>
                            <svg width="20" height="14" viewBox="0 0 22 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                                <path d="M3.5 8C4.59202 9.04403 7.54398 10.3978 13.5068 9.93754M1.25349 5.39919C2.77722 0.413397 8.08911 0.79692 10.9673 1.24436C14.2687 1.71311 20.8969 3.82675 20.9985 8.53129C21.1255 14.412 13.1894 15.3069 10.0784 14.9233C6.96748 14.5398 -0.46071 13.0696 1.25349 5.39919Z" stroke="#838384" stroke-width="1.5" stroke-linecap="round"/>
                            </svg>
                        </footer>
                    </main>
                </body>
            </html>"##,
        )
        .build(&window)
        .context("Failed to create webview")?;

    window.set_outer_position(PhysicalPosition::new(500f64, 500f64));

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

    // handle window actions
    tokio::spawn(async move {
        while let Some(action) = rx.recv().await {
            match action {
                WindowAction::Show => {
                    let _ = unsafe {
                        ShowWindow(
                            HWND(window.hwnd() as *mut std::ffi::c_void),
                            SW_SHOWNOACTIVATE,
                        )
                    };
                }
                WindowAction::Hide => {
                    let _ = unsafe {
                        ShowWindow(HWND(window.hwnd() as *mut std::ffi::c_void), SW_HIDE)
                    };
                }
                WindowAction::SetPosition {
                    top,
                    left,
                    bottom,
                    right,
                } => {
                    let mut x = left - 15;
                    let mut y = bottom;

                    let monitor = unsafe {
                        MonitorFromRect(
                            &RECT {
                                left,
                                top,
                                right,
                                bottom,
                            } as *const _,
                            MONITOR_DEFAULTTONEAREST,
                        )
                    };

                    let mut monitor_info = MONITORINFO::default();
                    monitor_info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;

                    unsafe {
                        let _ = GetMonitorInfoW(monitor, &mut monitor_info);
                    }

                    // If the bottom of the candidate window is hidden, show it above
                    y = if y + window.inner_size().height as i32 > monitor_info.rcWork.bottom {
                        top - window.inner_size().height as i32
                    } else {
                        y
                    };

                    // If the right of the candidate window is hidden, show it to the left
                    x = if x + window.inner_size().width as i32 > monitor_info.rcWork.right {
                        monitor_info.rcWork.right - window.inner_size().width as i32
                    } else {
                        x
                    };

                    // If the left of the candidate window is hidden, show it to the right
                    x = if x < monitor_info.rcWork.left {
                        monitor_info.rcWork.left
                    } else {
                        x
                    };

                    unsafe {
                        let _ = SetWindowPos(
                            HWND(window.hwnd() as *mut std::ffi::c_void),
                            HWND_TOPMOST,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                    window.set_outer_position(PhysicalPosition::new(x as f64, y as f64));
                }
                WindowAction::SetCandidate { candidates } => {
                    let max_len = candidates
                        .iter()
                        .map(|s| s.chars().count())
                        .max()
                        .unwrap_or(0) as u32;
                    window.set_inner_size(PhysicalSize::new(max(225, 120 + max_len * 18), 275));

                    let candidates = serde_json::to_string(&candidates)
                        .context("Failed to serialize candidates")
                        .unwrap();

                    event_loop_proxy
                        .send_event(format!("updateCandidates({})", candidates))
                        .unwrap();
                }
                WindowAction::SetSelection { index } => {
                    event_loop_proxy
                        .send_event(format!("updateSelection({})", index))
                        .unwrap();
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
            Event::UserEvent(script) => {
                webview.evaluate_script(&script).unwrap();
            }
            _ => (),
        }
    });
}
