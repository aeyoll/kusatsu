use yew::prelude::*;

#[function_component]
pub fn Back() -> Html {
    let reset = {
        Callback::from(move |_| {
            // Redirect to upload page
            let window = web_sys::window().unwrap();
            let _ = window.location().set_href("/");
        })
    };

    html! {
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
    }
}
