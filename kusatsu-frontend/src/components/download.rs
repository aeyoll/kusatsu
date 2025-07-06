use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::services::api::ApiClient;
use crate::utils::file_utils;

#[derive(Clone, PartialEq)]
enum DownloadState {
    Loading,
    Ready {
        filename: String,
        size: usize,
        is_encrypted: bool,
    },
    Error(String),
}

#[derive(Properties, PartialEq)]
pub struct DownloadProps {
    pub file_id: String,
}

#[function_component(Download)]
pub fn download(props: &DownloadProps) -> Html {
    let file_id = props.file_id.clone();
    let state = use_state(|| DownloadState::Loading);
    let encryption_key = use_state(|| None::<String>);
    let file_info = use_state(|| None::<crate::services::api::FileInfo>);

    {
        let state = state.clone();
        let file_info = file_info.clone();
        let encryption_key = encryption_key.clone();

        use_effect_with((), move |_| {
            let window = web_sys::window().unwrap();
            let location = window.location();

            let key_from_url = if let Ok(hash) = location.hash() {
                if hash.len() > 1 {
                    // Remove the # prefix
                    let key_str = &hash[1..];
                    Some(key_str.to_string())
                } else {
                    None
                }
            } else {
                None
            };

            encryption_key.set(key_from_url.clone());

            spawn_local(async move {
                let api_client = ApiClient::new();

                match api_client
                    .get_file_info(&file_id, key_from_url.as_deref())
                    .await
                {
                    Ok(info) => {
                        let max_downloads = info.max_downloads;
                        let download_count = info.download_count;
                        let is_expired = info.expires_at.is_some()
                            && info.expires_at.unwrap() < chrono::Utc::now();

                        file_info.set(Some(info.clone()));

                        if max_downloads.is_some() && max_downloads.unwrap() == download_count {
                            state.set(DownloadState::Error(format!(
                                "The maximum download limit of {} has been reached",
                                max_downloads.unwrap()
                            )));
                        } else if is_expired {
                            state.set(DownloadState::Error("The file has expired".to_string()));
                        } else {
                            state.set(DownloadState::Ready {
                                filename: info.filename,
                                size: info.original_size as usize,
                                is_encrypted: info.is_encrypted,
                            });
                        }
                    }
                    Err(e) => {
                        state.set(DownloadState::Error(format!(
                            "Failed to load file info: {}",
                            e
                        )));
                    }
                }
            });

            || ()
        });
    }

    let reset = {
        Callback::from(move |_| {
            // Redirect to upload page
            let window = web_sys::window().unwrap();
            let _ = window.location().set_href("/");
        })
    };

    let api_client = ApiClient::new();
    let base_url = api_client.base_url;

    html! {
        <div class="max-w-2xl mx-auto bg-white dark:bg-gray-800 rounded-xl shadow-lg p-8">
            <h2 class="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-8 text-center">
                {"📥 Download File"}
            </h2>

            {match &*state {
                DownloadState::Loading => html! {
                    <div class="flex flex-col items-center justify-center py-12">
                        <div class="w-16 h-16 mb-4">
                            <svg class="w-full h-full text-blue-500 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                            </svg>
                        </div>
                        <p class="text-lg font-medium text-gray-700 dark:text-gray-300">{"Loading file information..."}</p>
                    </div>
                },

                DownloadState::Ready { filename, size, .. } => html! {
                    <div class="space-y-6">
                        // File preview card
                        <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-6">
                            <div class="flex items-center space-x-4">
                                <span class="text-4xl">{file_utils::get_file_icon(filename, None)}</span>
                                <div class="flex-1 min-w-0">
                                    <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 truncate">{filename}</h3>
                                    <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">
                                        {file_utils::format_file_size(*size)}
                                    </p>
                                </div>
                            </div>
                        </div>

                        // Download section
                        <div class="space-y-4">
                            {
                                html! {
                                    <div class="space-y-4">
                                        <div class="p-4 bg-blue-50 dark:bg-blue-900/50 border border-blue-200 dark:border-blue-800 rounded-lg">
                                            <div class="flex items-center">
                                                <svg class="w-5 h-5 text-blue-400 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                                                </svg>
                                                <p class="text-blue-800 dark:text-blue-300 text-sm">
                                                    {"File is ready to download."}
                                                </p>
                                            </div>
                                        </div>
                                        <form method="POST" action={format!("{}/api/files/{}/form", base_url, props.file_id)}>
                                            <input type="hidden" name="encryption_key" value={
                                                if let Some(key) = &*encryption_key {
                                                    key.clone()
                                                } else {
                                                    "".to_string()
                                                }
                                            } />
                                            <button
                                                type="submit"
                                                class="w-full bg-blue-600 text-white py-3 px-6 rounded-lg hover:bg-blue-700 transition-colors font-medium text-lg flex items-center justify-center space-x-2"
                                            >
                                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                                </svg>
                                                <span>{"Download File"}</span>
                                            </button>
                                        </form>
                                    </div>
                                }
                            }
                        </div>

                        // Back to upload button
                        <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
                            <button
                                class="w-full bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 py-2 px-4 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors font-medium flex items-center justify-center space-x-2"
                                onclick={reset}
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                                </svg>
                                <span>{"Upload Another File"}</span>
                            </button>
                        </div>
                    </div>
                },

                DownloadState::Error(error) => html! {
                    <div class="space-y-6">
                        // Error display
                        <div class="text-center py-8">
                            <div class="mx-auto w-16 h-16 mb-4">
                                <svg class="w-full h-full text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                                </svg>
                            </div>
                            <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">{"Download Failed"}</h3>
                        </div>

                        // Error message
                        <div class="p-4 bg-red-50 dark:bg-red-900/50 border border-red-200 dark:border-red-800 rounded-lg">
                            <div class="flex items-center">
                                <svg class="w-5 h-5 text-red-400 mr-3 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                                </svg>
                                <p class="text-red-800 dark:text-red-300 text-sm">{error}</p>
                            </div>
                        </div>

                        // Troubleshooting suggestions
                        <div class="space-y-3">
                            <h4 class="text-sm font-medium text-gray-900 dark:text-gray-100">{"Possible solutions:"}</h4>
                            <ul class="text-sm text-gray-600 dark:text-gray-400 space-y-2">
                                <li class="flex items-start">
                                    <span class="text-gray-400 dark:text-gray-500 mr-2">{"•"}</span>
                                    {"Check if the download link is correct"}
                                </li>
                                <li class="flex items-start">
                                    <span class="text-gray-400 dark:text-gray-500 mr-2">{"•"}</span>
                                    {"The file may have expired"}
                                </li>
                                <li class="flex items-start">
                                    <span class="text-gray-400 dark:text-gray-500 mr-2">{"•"}</span>
                                    {"The maximum download limit may have been reached"}
                                </li>
                                <li class="flex items-start">
                                    <span class="text-gray-400 dark:text-gray-500 mr-2">{"•"}</span>
                                    {"Contact the sender for a new link"}
                                </li>
                            </ul>
                        </div>

                        // Back to upload button
                        <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
                            <button
                                class="w-full bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 py-2 px-4 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors font-medium flex items-center justify-center space-x-2"
                                onclick={reset}
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                                </svg>
                                <span>{"Upload Another File"}</span>
                            </button>
                        </div>
                    </div>
                },
            }}
        </div>
    }
}
