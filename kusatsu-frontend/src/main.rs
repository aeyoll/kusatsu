use yew::prelude::*;
use yew_router::prelude::*;

mod components;
mod services;
mod utils;

use components::{download::Download, header::Header, upload::Upload};
use utils::url_utils;

#[derive(Clone, Routable, PartialEq)]
pub enum AppRoute {
    #[at("/")]
    Upload,
    #[at("/download/:file_id")]
    Download { file_id: String },
}

#[derive(Clone, PartialEq)]
pub struct UploadResult {
    pub file_id: String,
    pub download_url: String,
    pub encryption_key: String,
    pub curl_command: String,
}

#[function_component(App)]
pub fn app() -> Html {
    let current_route = use_state(|| AppRoute::Upload);
    let upload_result = use_state(|| None::<UploadResult>);

    let on_navigate = {
        let current_route = current_route.clone();
        Callback::from(move |route: AppRoute| {
            current_route.set(route);
        })
    };

    let on_upload_complete = {
        let upload_result = upload_result.clone();
        Callback::from(
            move |(file_id, download_url, encryption_key, curl_command): (String, String, String, String)| {
                upload_result.set(Some(UploadResult {
                    file_id,
                    download_url,
                    encryption_key,
                    curl_command,
                }));
            },
        )
    };

    let switch = {
        let upload_result = upload_result.clone();
        let on_upload_complete = on_upload_complete.clone();

        move |routes: AppRoute| -> Html {
            match routes {
                AppRoute::Upload => html! {
                    <div>
                        <Upload on_upload_complete={on_upload_complete.clone()} />
                    </div>
                },
                AppRoute::Download { file_id } => html! { <Download {file_id} /> },
            }
        }
    };

    html! {
        <BrowserRouter>
            <div class="app">
                <Header current_route={(*current_route).clone()} {on_navigate} />

                <main class="main-content">
                    <Switch<AppRoute> render={switch} />
                </main>
            </div>
        </BrowserRouter>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
