use crate::{
    services::api::{ApiClient, StartUploadRequest},
    utils::url_utils,
};
use gloo::file::File;
use web_sys::{DragEvent, Event, HtmlInputElement};
use yew::prelude::*;

// Constants
const MAX_SINGLE_UPLOAD_SIZE: usize = 5 * 1024 * 1024; // 5MB
const CHUNK_SIZE: i32 = 5 * 1024 * 1024; // 5MB chunks
const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024 * 1024; // 5GB max

#[derive(Clone, PartialEq)]
pub enum UploadState {
    Idle,
    Preparing,
    StartingUpload,
    UploadingChunks {
        upload_id: String,
        uploaded_chunks: i32,
        total_chunks: i32,
        current_chunk: i32,
        progress: f32,
    },
    Completing,
    Completed {
        file_id: String,
        download_url: String,
        encryption_key: String,
        curl_command: String,
    },
    Error(String),
}

#[derive(Properties, PartialEq)]
pub struct UploadProps {
    pub on_upload_complete: Callback<(String, String, String, String)>, // file_id, download_url, encryption_key, curl_command
}

#[function_component(Upload)]
pub fn upload(props: &UploadProps) -> Html {
    let file_input_ref = use_node_ref();
    let selected_file = use_state(|| None::<File>);
    let upload_state = use_state(|| UploadState::Idle);
    let expires_in_hours = use_state(|| 24i32);
    let max_downloads = use_state(|| None::<i32>);
    let enable_max_downloads = use_state(|| false);
    let api_client = use_state(ApiClient::new);
    let drag_over = use_state(|| false);

    let on_file_select = {
        let selected_file = selected_file.clone();
        let upload_state = upload_state.clone();

        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(files) = input.files() {
                if files.length() > 0 {
                    if let Some(file) = files.get(0) {
                        let file = File::from(file);

                        // Check file size
                        if file.size() > MAX_FILE_SIZE {
                            upload_state.set(UploadState::Error(format!(
                                "File too large. Maximum size is {} MB.",
                                MAX_FILE_SIZE / (1024 * 1024)
                            )));
                            return;
                        }

                        selected_file.set(Some(file));
                        upload_state.set(UploadState::Idle);
                    }
                }
            }
        })
    };

    let on_drag_over = {
        let drag_over = drag_over.clone();
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            drag_over.set(true);
        })
    };

    let on_drag_leave = {
        let drag_over = drag_over.clone();
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            drag_over.set(false);
        })
    };

    let on_drop = {
        let selected_file = selected_file.clone();
        let upload_state = upload_state.clone();
        let drag_over = drag_over.clone();

        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            drag_over.set(false);

            if let Some(files) = e.data_transfer().and_then(|dt| dt.files()) {
                if files.length() > 0 {
                    if let Some(file) = files.get(0) {
                        let file = File::from(file);

                        // Check file size
                        if file.size() > MAX_FILE_SIZE {
                            upload_state.set(UploadState::Error(format!(
                                "File too large. Maximum size is {} MB.",
                                MAX_FILE_SIZE / (1024 * 1024)
                            )));
                            return;
                        }

                        selected_file.set(Some(file));
                        upload_state.set(UploadState::Idle);
                    }
                }
            }
        })
    };

    let trigger_file_input = {
        let file_input_ref = file_input_ref.clone();
        Callback::from(move |_| {
            if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                input.click();
            }
        })
    };

    let start_upload = {
        let selected_file = selected_file.clone();
        let upload_state = upload_state.clone();
        let expires_in_hours = expires_in_hours.clone();
        let max_downloads = max_downloads.clone();
        let enable_max_downloads = enable_max_downloads.clone();
        let api_client = api_client.clone();
        let on_upload_complete = props.on_upload_complete.clone();

        Callback::from(move |_| {
            if let Some(file) = (*selected_file).clone() {
                let upload_state = upload_state.clone();
                let api_client = (*api_client).clone();
                let on_upload_complete = on_upload_complete.clone();
                let expires_in_hours = *expires_in_hours;
                let max_downloads = if *enable_max_downloads {
                    *max_downloads
                } else {
                    None
                };

                wasm_bindgen_futures::spawn_local(async move {
                    upload_state.set(UploadState::Preparing);

                    let file_size = file.size() as usize;
                    let filename = file.name();
                    let mime_type = if file.raw_mime_type().is_empty() {
                        None
                    } else {
                        Some(file.raw_mime_type())
                    };

                    // Decide between chunked and single upload
                    if file_size <= MAX_SINGLE_UPLOAD_SIZE {
                        // Use single upload for smaller files
                        match perform_single_upload(
                            &api_client,
                            file,
                            filename,
                            mime_type,
                            Some(expires_in_hours),
                            max_downloads,
                            upload_state.clone(),
                        )
                        .await
                        {
                            Ok((file_id, download_url, encryption_key, curl_command)) => {
                                upload_state.set(UploadState::Completed {
                                    file_id: file_id.clone(),
                                    download_url: download_url.clone(),
                                    encryption_key: encryption_key.clone(),
                                    curl_command: curl_command.clone(),
                                });
                                on_upload_complete.emit((
                                    file_id,
                                    download_url,
                                    encryption_key,
                                    curl_command,
                                ));
                            }
                            Err(error) => {
                                upload_state.set(UploadState::Error(error));
                            }
                        }
                    } else {
                        // Use chunked upload for larger files
                        match perform_chunked_upload(
                            &api_client,
                            file,
                            filename,
                            mime_type,
                            Some(expires_in_hours),
                            max_downloads,
                            upload_state.clone(),
                        )
                        .await
                        {
                            Ok((file_id, download_url, encryption_key, curl_command)) => {
                                upload_state.set(UploadState::Completed {
                                    file_id: file_id.clone(),
                                    download_url: download_url.clone(),
                                    encryption_key: encryption_key.clone(),
                                    curl_command: curl_command.clone(),
                                });
                                on_upload_complete.emit((
                                    file_id,
                                    download_url,
                                    encryption_key,
                                    curl_command,
                                ));
                            }
                            Err(error) => {
                                upload_state.set(UploadState::Error(error));
                            }
                        }
                    }
                });
            }
        })
    };

    let clear_file = {
        let selected_file = selected_file.clone();
        let upload_state = upload_state.clone();
        let file_input_ref = file_input_ref.clone();

        Callback::from(move |_| {
            selected_file.set(None);
            upload_state.set(UploadState::Idle);
            if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                input.set_value("");
            }
        })
    };

    let cancel_upload = {
        let upload_state = upload_state.clone();
        let selected_file = selected_file.clone();
        let file_input_ref = file_input_ref.clone();

        Callback::from(move |_| {
            upload_state.set(UploadState::Idle);
            selected_file.set(None);
            if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                input.set_value("");
            }
        })
    };

    let on_expires_change = {
        let expires_in_hours = expires_in_hours.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Ok(value) = input.value().parse::<i32>() {
                expires_in_hours.set(value);
            }
        })
    };

    let on_max_downloads_toggle = {
        let enable_max_downloads = enable_max_downloads.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            enable_max_downloads.set(input.checked());
        })
    };

    let on_max_downloads_change = {
        let max_downloads = max_downloads.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Ok(value) = input.value().parse::<i32>() {
                max_downloads.set(Some(value));
            }
        })
    };

    let is_uploading = matches!(
        *upload_state,
        UploadState::Preparing
            | UploadState::StartingUpload
            | UploadState::UploadingChunks { .. }
            | UploadState::Completing
    );

    html! {
        <div class="max-w-2xl mx-auto bg-white dark:bg-gray-800 rounded-xl shadow-lg p-8">
            <h2 class="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-8 text-center">{"Upload and attach file"}</h2>

            // Hidden file input
            <input
                ref={file_input_ref}
                type="file"
                class="hidden"
                onchange={on_file_select}
                disabled={is_uploading}
            />

            // Upload area
            <div
                class={format!(
                    "relative border-2 border-dashed rounded-xl p-12 text-center transition-all duration-200 cursor-pointer {}",
                    if *drag_over {
                        "border-blue-400 bg-blue-50 dark:bg-blue-900/50"
                    } else if is_uploading {
                        "border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-700 cursor-not-allowed"
                    } else {
                        "border-gray-300 dark:border-gray-600 hover:border-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/50"
                    }
                )}
                ondragover={on_drag_over}
                ondragleave={on_drag_leave}
                ondrop={on_drop}
                onclick={if !is_uploading { trigger_file_input } else { Callback::noop() }}
            >
                // Upload icon
                <div class="mx-auto w-16 h-16 mb-4">
                    <svg class="w-full h-full text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                    </svg>
                </div>

                <div class="space-y-2">
                    <p class="text-lg font-medium text-gray-700 dark:text-gray-300">
                        if is_uploading {
                            {"Uploading..."}
                        } else {
                            {"Click to Upload or drag and drop"}
                        }
                    </p>
                    <p class="text-sm text-gray-500 dark:text-gray-400">
                        {format!("(Max. File size: {} MB)", MAX_FILE_SIZE / (1024 * 1024))}
                    </p>
                </div>
            </div>

            // File preview and progress section
            if let Some(file) = (*selected_file).as_ref() {
                <div class="mt-8 space-y-4">
                    <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-4">
                        <div class="flex items-center justify-between">
                            <div class="flex items-center space-x-3 flex-1 min-w-0">
                                <span class="text-2xl">{
                                    {
                                        let mime_type = file.raw_mime_type();
                                        crate::utils::file_utils::get_file_icon(
                                            &file.name(),
                                            if mime_type.is_empty() { None } else { Some(&mime_type) }
                                        )
                                    }
                                }</span>
                                <div class="flex-1 min-w-0">
                                    <p class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">{&file.name()}</p>
                                    <p class="text-sm text-gray-500 dark:text-gray-400">
                                        {format!("{} • {}", crate::utils::file_utils::format_file_size(file.size() as usize),
                                            match &*upload_state {
                                                UploadState::Completed { .. } => "Completed",
                                                UploadState::Error(_) => "Error",
                                                UploadState::Preparing => "Preparing...",
                                                UploadState::StartingUpload => "Starting...",
                                                UploadState::UploadingChunks { .. } => "Uploading...",
                                                UploadState::Completing => "Finalizing...",
                                                _ => "Ready"
                                            }
                                        )}
                                    </p>
                                </div>
                            </div>

                            // Action button
                            <button
                                class="ml-4 p-2 text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
                                onclick={if is_uploading { cancel_upload } else { clear_file }}
                                title={if is_uploading { "Cancel upload" } else { "Remove file" }}
                            >
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                </svg>
                            </button>
                        </div>

                        // Progress bar
                        {match &*upload_state {
                            UploadState::UploadingChunks { progress, uploaded_chunks, total_chunks, .. } => html! {
                                <div class="mt-3">
                                    <div class="flex justify-between text-xs text-gray-600 dark:text-gray-400 mb-1">
                                        <span>{format!("Chunk {} of {}", uploaded_chunks + 1, total_chunks)}</span>
                                        <span>{format!("{:.0}%", progress * 100.0)}</span>
                                    </div>
                                    <div class="w-full bg-gray-200 dark:bg-gray-600 rounded-full h-2">
                                        <div
                                            class="bg-blue-600 h-2 rounded-full transition-all duration-300"
                                            style={format!("width: {:.1}%", progress * 100.0)}
                                        ></div>
                                    </div>
                                </div>
                            },
                            UploadState::Preparing | UploadState::StartingUpload | UploadState::Completing => html! {
                                <div class="mt-3">
                                    <div class="w-full bg-gray-200 dark:bg-gray-600 rounded-full h-2">
                                        <div class="bg-blue-600 h-2 rounded-full animate-pulse w-1/3"></div>
                                    </div>
                                </div>
                            },
                            UploadState::Completed { .. } => html! {
                                <div class="mt-3">
                                    <div class="w-full bg-green-200 dark:bg-green-800 rounded-full h-2">
                                        <div class="bg-green-600 h-2 rounded-full w-full"></div>
                                    </div>
                                </div>
                            },
                            _ => html! {}
                        }}
                    </div>
                </div>
            }

            // Error display
            if let UploadState::Error(error) = &*upload_state {
                <div class="mt-6 p-4 bg-red-50 dark:bg-red-900/50 border border-red-200 dark:border-red-800 rounded-lg">
                    <div class="flex items-center">
                        <svg class="w-5 h-5 text-red-400 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                        </svg>
                        <p class="text-red-800 dark:text-red-300 text-sm">{error}</p>
                    </div>
                </div>
            }

            // Success display
            if let UploadState::Completed { file_id: _, download_url, encryption_key: _, curl_command } = &*upload_state {
                <div class="mt-6 p-4 bg-green-50 dark:bg-green-900/50 border border-green-200 dark:border-green-800 rounded-lg">
                    <div class="flex items-center">
                        <svg class="w-5 h-5 text-green-400 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                        </svg>
                        <p class="text-green-800 dark:text-green-300 text-sm">
                            {"File uploaded successfully!"  }
                        </p>
                    </div>
                </div>

                <div class="mt-4">
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        {"Shareable URL:"}
                    </label>
                    <div class="flex">
                        <input
                            type="text"
                            value={download_url.clone()}
                            readonly=true
                            class="flex-1 p-2 border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 rounded-l text-sm font-mono"
                        />
                        <button
                            class="px-4 py-2 bg-blue-600 text-white rounded-r hover:bg-blue-700 text-sm"
                            onclick={
                                let url = download_url.clone();
                                Callback::from(move |_| {
                                    let url = url.clone();
                                    wasm_bindgen_futures::spawn_local(async move {
                                        if let Err(e) = url_utils::copy_to_clipboard(&url).await {
                                            web_sys::console::log_1(&format!("Failed to copy: {:?}", e).into());
                                        }
                                    });
                                })
                            }
                        >
                            {"Copy"}
                        </button>
                    </div>
                </div>

                <div class="mt-4">
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        {"Download with curl:"}
                    </label>
                    <div class="flex">
                        <input
                            type="text"
                            value={curl_command.clone()}
                            readonly=true
                            class="flex-1 p-2 border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 rounded-l text-sm font-mono text-xs"
                        />
                        <button
                            class="px-4 py-2 bg-gray-600 text-white rounded-r hover:bg-gray-700 text-sm"
                            onclick={
                                let curl_command = curl_command.clone();

                                Callback::from(move |_| {
                                    let cmd = curl_command.clone();
                                    wasm_bindgen_futures::spawn_local(async move {
                                        if let Err(e) = url_utils::copy_to_clipboard(&cmd).await {
                                            web_sys::console::log_1(&format!("Failed to copy: {:?}", e).into());
                                        }
                                    });
                                })
                            }
                        >
                            {"Copy"}
                        </button>
                    </div>
                </div>
            }

            // Upload options
            if selected_file.is_some() && !matches!(*upload_state, UploadState::Completed { .. }) {
                <div class="mt-8 space-y-6">
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                {"Expires in (hours)"}
                            </label>
                            <input
                                type="number"
                                min="1"
                                max="8760"
                                value={expires_in_hours.to_string()}
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                onchange={on_expires_change}
                                disabled={is_uploading}
                            />
                        </div>

                        <div class="flex flex-col">
                            <label class="flex items-center mb-2">
                                <input
                                    type="checkbox"
                                    checked={*enable_max_downloads}
                                    class="mr-2 rounded"
                                    onchange={on_max_downloads_toggle}
                                    disabled={is_uploading}
                                />
                                <span class="text-sm font-medium text-gray-700 dark:text-gray-300">{"Limit downloads"}</span>
                            </label>

                            if *enable_max_downloads {
                                <input
                                    type="number"
                                    min="1"
                                    max="1000"
                                    value={max_downloads.map(|n| n.to_string()).unwrap_or_else(|| "1".to_string())}
                                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                    placeholder="Maximum downloads"
                                    onchange={on_max_downloads_change}
                                    disabled={is_uploading}
                                />
                            }
                        </div>
                    </div>

                    // Upload button
                    <button
                        class="w-full bg-blue-600 text-white py-3 px-6 rounded-lg hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors font-medium text-lg"
                        onclick={start_upload}
                        disabled={is_uploading}
                    >
                        {match &*upload_state {
                            UploadState::Idle => "Upload File",
                            UploadState::Preparing => "Preparing upload...",
                            UploadState::StartingUpload => "Starting upload...",
                            UploadState::UploadingChunks { .. } => "Uploading...",
                            UploadState::Completing => "Finalizing...",
                            UploadState::Completed { .. } => "Upload Complete",
                            UploadState::Error(_) => "Retry Upload",
                        }}
                    </button>
                </div>
            }
        </div>
    }
}

// Single upload for smaller files
async fn perform_single_upload(
    api_client: &ApiClient,
    file: File,
    filename: String,
    mime_type: Option<String>,
    expires_in_hours: Option<i32>,
    max_downloads: Option<i32>,
    _upload_state: UseStateHandle<UploadState>,
) -> Result<(String, String, String, String), String> {
    // Read file data
    let file_data = gloo::file::futures::read_as_bytes(&file)
        .await
        .map_err(|e| format!("Failed to read file: {:?}", e))?;

    // Upload file
    let response = api_client
        .upload_file(
            file_data,
            filename,
            mime_type,
            expires_in_hours,
            max_downloads,
        )
        .await
        .map_err(|e| format!("Upload failed: {:?}", e))?;

    Ok((
        response.file_id.to_string(),
        response.download_url,
        response.encryption_key.unwrap_or_else(|| "".to_string()),
        response.curl_command,
    ))
}

// Chunked upload for larger files
async fn perform_chunked_upload(
    api_client: &ApiClient,
    file: File,
    filename: String,
    mime_type: Option<String>,
    expires_in_hours: Option<i32>,
    max_downloads: Option<i32>,
    upload_state: UseStateHandle<UploadState>,
) -> Result<(String, String, String, String), String> {
    // Start upload session
    upload_state.set(UploadState::StartingUpload);

    let start_request = StartUploadRequest {
        filename: filename.clone(),
        file_size: file.size() as i64,
        mime_type,
        chunk_size: Some(CHUNK_SIZE),
        expires_in_hours,
        max_downloads,
    };

    let start_response = api_client
        .start_chunked_upload(start_request)
        .await
        .map_err(|e| format!("Failed to start upload: {:?}", e))?;

    let upload_id = start_response.upload_id;
    let total_chunks = start_response.total_chunks;
    let chunk_size = start_response.chunk_size as usize;

    // Upload chunks by reading file in chunks (don't load entire file into memory)
    for chunk_number in 0..total_chunks {
        let start_offset = chunk_number as usize * chunk_size;
        let end_offset = std::cmp::min(start_offset + chunk_size, file.size() as usize);

        upload_state.set(UploadState::UploadingChunks {
            upload_id: upload_id.to_string(),
            uploaded_chunks: chunk_number,
            total_chunks,
            current_chunk: chunk_number,
            progress: chunk_number as f32 / total_chunks as f32,
        });

        // Read only the chunk we need (not the entire file)
        let chunk_data = read_file_chunk(&file, start_offset, end_offset - start_offset)
            .await
            .map_err(|e| format!("Failed to read chunk {}: {:?}", chunk_number, e))?;

        let _chunk_response = api_client
            .upload_chunk(&upload_id.to_string(), chunk_number, &chunk_data)
            .await
            .map_err(|e| format!("Failed to upload chunk {}: {:?}", chunk_number, e))?;
    }

    // Complete upload
    upload_state.set(UploadState::Completing);

    let complete_response = api_client
        .complete_chunked_upload(&upload_id.to_string())
        .await
        .map_err(|e| format!("Failed to complete upload: {:?}", e))?;

    Ok((
        complete_response.file_id.to_string(),
        complete_response.download_url,
        complete_response
            .encryption_key
            .unwrap_or_else(|| "".to_string()),
        complete_response.curl_command,
    ))
}

// Helper function to read a specific chunk of a file without loading the entire file
async fn read_file_chunk(file: &File, start: usize, length: usize) -> Result<Vec<u8>, String> {
    use gloo::file::Blob;

    // Create a file slice for the specific chunk
    let end = start + length;
    let web_file: &web_sys::File = file.as_ref();
    let web_blob_slice = web_file
        .slice_with_i32_and_i32(start as i32, end as i32)
        .map_err(|e| format!("Failed to slice file: {:?}", e))?;

    // Convert web_sys::Blob to gloo::file::Blob
    let gloo_blob = Blob::from(web_blob_slice);

    // Convert blob slice to ArrayBuffer using the existing gloo functionality
    let array_buffer = gloo::file::futures::read_as_array_buffer(&gloo_blob)
        .await
        .map_err(|e| format!("Failed to read chunk: {:?}", e))?;

    // Convert ArrayBuffer to Vec<u8>
    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    Ok(uint8_array.to_vec())
}
