use anyhow::{Context as _, Result};
use tao::{
    dpi::{PhysicalPosition, PhysicalSize},
    event_loop::EventLoop,
    platform::windows::{WindowBuilderExtWindows, WindowExtWindows},
    window::{Window, WindowBuilder},
};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        SetWindowLongW, GWL_EXSTYLE, GWL_STYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
        WS_POPUP,
    },
};
use wry::{WebView, WebViewBuilder};

use crate::UserEvent;

pub fn create_candidate_window(event_loop: &EventLoop<UserEvent>) -> Result<Window> {
    let window = WindowBuilder::new()
        .with_decorations(false)
        .with_title("CandidateList")
        .with_focused(false)
        .with_visible(false)
        .with_undecorated_shadow(false)
        .with_transparent(true)
        .build(&event_loop)
        .context("Failed to create window")?;

    window.set_inner_size(PhysicalSize::new(275.0, 275.0));
    window.set_outer_position(PhysicalPosition::new(500f64, 500f64));

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

    Ok(window)
}

pub fn create_candidate_webview(window: &Window) -> Result<WebView> {
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

    Ok(webview)
}
