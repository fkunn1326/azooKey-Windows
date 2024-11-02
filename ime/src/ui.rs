use std::sync::mpsc::Receiver;

use anyhow::{Context, Result};
use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::platform::windows::{EventLoopBuilderExtWindows, WindowExtWindows};
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        SetWindowLongW, ShowWindow, GWL_EXSTYLE, GWL_STYLE, SW_SHOWNOACTIVATE, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    },
};
use wry::WebViewBuilder;

pub struct CandidateList;

#[derive(Debug)]
pub struct LocateEvent {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug)]
pub struct CandidateEvent {
    pub candidates: Vec<String>,
}

pub enum UiEvent {
    Locate(LocateEvent),
    Candidate(CandidateEvent),
    Show,
    Hide,
}

impl CandidateList {
    pub fn create(rx: Receiver<UiEvent>) -> Result<Self> {
        let event_loop = EventLoopBuilder::<String>::with_user_event()
            .with_any_thread(true)
            .build();
        let window = WindowBuilder::new()
            .with_title("CandidateList")
            .with_focused(false)
            .with_visible(false)
            .build(&event_loop)
            .context("Failed to create window")?;

        // set size
        window.set_inner_size(PhysicalSize::new(300.0, 250.0));

        // set z-order
        window.set_always_on_top(true);

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

        // unsafe {
        //     let _ = ShowWindow(HWND(hwnd), SW_SHOWNOACTIVATE);
        // }

        let webview = WebViewBuilder::new(&window)
            .with_html(
                r#"
            <html>
                <body style="background: #D2D2D2">
                    <ol>
                        <li></li>
                        <li></li>
                        <li></li>
                        <li></li>
                        <li></li>
                    </ol>
                    <script>
                        function update(str) {
                            const newItems = str.split(',').map(item => item.trim());

                            const listItems = document.querySelectorAll('li');

                            listItems.forEach((item, index) => {
                                if (index < newItems.length) {
                                    item.textContent = newItems[index];
                                }
                            });
                        }
                    </script>
                </body>
            </html>"#,
            )
            .build()
            .context("Failed to create webview")?;

        // event loopとは別スレッドでメッセージを受け取る
        let event_loop_proxy = event_loop.create_proxy();

        std::thread::spawn(move || loop {
            let result: Result<()> = (|| {
                let message = rx.recv()?;
                match message {
                    UiEvent::Locate(event) => window.set_outer_position(PhysicalPosition::new(
                        event.x as f64,
                        (event.y + 50_i32) as f64,
                    )),
                    UiEvent::Candidate(event) => {
                        event_loop_proxy.send_event(event.candidates[0..5].join(","))?;
                    }
                    UiEvent::Show => {
                        // let _ = ShowWindow(HWND(hwnd), SW_SHOWNOACTIVATE);
                        // window.set_visible(true);
                        let _ = unsafe {
                            ShowWindow(
                                HWND(window.hwnd() as *mut std::ffi::c_void),
                                SW_SHOWNOACTIVATE,
                            )
                        };
                    }
                    UiEvent::Hide => {
                        // let _ = ShowWindow(HWND(hwnd), 0);
                        window.set_visible(false);
                    }
                }

                Ok(())
            })();

            if let Err(err) = result {
                let _ = crate::utils::winutils::alert(&format!("Error: {:?}", err));
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
                Event::UserEvent(data) => {
                    let _ = webview.evaluate_script(&format!("update('{}')", data));
                }
                _ => (),
            }
        });
    }
}
