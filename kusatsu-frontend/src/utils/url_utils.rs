use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, HtmlInputElement, HtmlElement, HtmlDocument};

pub async fn copy_to_clipboard(text: &str) -> Result<(), JsValue> {
    let window = window().ok_or("no global `window` exists")?;

    // Try modern Clipboard API first
    let navigator = window.navigator();
    if let Ok(clipboard) = js_sys::Reflect::get(&navigator, &"clipboard".into()) {
        if !clipboard.is_undefined() {
            let clipboard: web_sys::Clipboard = clipboard.dyn_into()?;
            let promise = clipboard.write_text(text);
            match JsFuture::from(promise).await {
                Ok(_) => return Ok(()),
                Err(_) => {
                    // Fall through to fallback method
                }
            }
        }
    }

    // Fallback method using a temporary input element
    let document = window.document().ok_or("no document")?;
    let input = document
        .create_element("input")?
        .dyn_into::<HtmlInputElement>()?;

    input.set_value(text);

    // Style the input element (cast to HtmlElement to access style)
    let input_element: &HtmlElement = input.as_ref();
    let style = input_element.style();
    style.set_property("position", "absolute")?;
    style.set_property("left", "-9999px")?;

    let body = document.body().ok_or("no body")?;
    body.append_child(&input)?;

    input.select();

    // Cast document to HtmlDocument to access exec_command
    let html_document: HtmlDocument = document.dyn_into()?;
    let success = html_document.exec_command("copy").unwrap_or(false);
    body.remove_child(&input)?;

    if success {
        Ok(())
    } else {
        Err("Failed to copy to clipboard".into())
    }
}
