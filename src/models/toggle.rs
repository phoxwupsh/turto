use poise::ChoiceParameter;

#[derive(ChoiceParameter)]
pub enum ToggleOption {
    #[name = "on"]
    On,
    #[name = "off"]
    Off,
}
