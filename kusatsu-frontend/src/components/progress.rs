use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ProgressProps {
    pub progress: f32, // 0.0 to 1.0
    pub message: AttrValue,
    pub show_percentage: bool,
}

impl Default for ProgressProps {
    fn default() -> Self {
        Self {
            progress: 0.0,
            message: "Processing...".into(),
            show_percentage: true,
        }
    }
}

#[function_component(Progress)]
pub fn progress(props: &ProgressProps) -> Html {
    let percentage = (props.progress * 100.0).round() as u32;
    let width_style = format!("width: {}%", percentage);

    html! {
        <div class="progress-container">
            <div class="progress-message">
                {props.message.to_string()}
            </div>

            <div class="progress-bar">
                <div class="progress-fill" style={width_style}></div>
            </div>

            if props.show_percentage {
                <div class="progress-percentage">
                    {format!("{}%", percentage)}
                </div>
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct SpinnerProps {
    pub message: Option<AttrValue>,
}

#[function_component(Spinner)]
pub fn spinner(props: &SpinnerProps) -> Html {
    html! {
        <div class="spinner-container">
            <div class="spinner"></div>
            if let Some(message) = &props.message {
                <div class="spinner-message">
                    {message.to_string()}
                </div>
            }
        </div>
    }
}
