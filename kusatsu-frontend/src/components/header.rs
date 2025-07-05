use crate::AppRoute;
use web_sys::{window, Document, Element};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    pub current_route: AppRoute,
    pub on_navigate: Callback<AppRoute>,
}

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    let dark_mode = use_state(|| {
        // Check if dark mode is enabled in localStorage or system preference
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(stored_mode)) = storage.get_item("dark-mode") {
                    return stored_mode == "true";
                }
            }
            // Fallback to system preference
            if let Some(media_query) = window
                .match_media("(prefers-color-scheme: dark)")
                .ok()
                .flatten()
            {
                return media_query.matches();
            }
        }
        false
    });

    // Apply dark mode class to document
    {
        let dark_mode = *dark_mode;
        use_effect_with(dark_mode, move |&is_dark| {
            if let Some(window) = window() {
                if let Some(document) = window.document() {
                    if let Some(html_element) = document.document_element() {
                        if is_dark {
                            let _ = html_element.class_list().add_1("dark");
                        } else {
                            let _ = html_element.class_list().remove_1("dark");
                        }
                    }
                }
                // Store preference in localStorage
                if let Ok(Some(storage)) = window.local_storage() {
                    let _ = storage.set_item("dark-mode", &is_dark.to_string());
                }
            }
        });
    }

    let navigate_to_upload = {
        let on_navigate = props.on_navigate.clone();
        Callback::from(move |_| on_navigate.emit(AppRoute::Upload))
    };

    let toggle_dark_mode = {
        let dark_mode = dark_mode.clone();
        Callback::from(move |_| {
            dark_mode.set(!*dark_mode);
        })
    };

    let open_github = Callback::from(move |_| {
        if let Some(window) = window() {
            let _ = window.open_with_url_and_target("https://github.com/aeyoll/kusatsu", "_blank");
        }
    });

    html! {
        <header class="header">
            <div class="header-content">
                <div class="logo">
                    <h1>{"Kusatsu"}</h1>
                </div>

                <nav class="nav">
                    <button
                        class={classes!("nav-btn", if props.current_route == AppRoute::Upload { "active" } else { "" })}
                        onclick={navigate_to_upload}
                        title="Upload files"
                    >
                        {"📤 Upload"}
                    </button>

                    <button
                        class="nav-btn github-btn"
                        onclick={open_github}
                        title="View on GitHub"
                    >
                        {"🐙 GitHub"}
                    </button>

                    <button
                        class="nav-btn theme-toggle"
                        onclick={toggle_dark_mode}
                        title={if *dark_mode { "Switch to light mode" } else { "Switch to dark mode" }}
                    >
                        {if *dark_mode { "☀️" } else { "🌙" }}
                    </button>
                </nav>
            </div>
        </header>
    }
}
