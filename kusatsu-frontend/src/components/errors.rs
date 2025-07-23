use yew::prelude::*;

use crate::components::back::Back;

#[derive(Properties, PartialEq)]
pub struct ErrorsProps {
    pub error: AttrValue,
}

#[function_component]
pub fn Errors(props: &ErrorsProps) -> Html {
    html! {
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
                    <p class="text-red-800 dark:text-red-300 text-sm">{props.error.to_string()}</p>
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

            <Back />
        </div>
    }
}
