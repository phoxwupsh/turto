use poise::ChoiceParameter;

#[derive(ChoiceParameter, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum ToggleOption {
    #[name = "on"]
    On,
    #[name = "off"]
    Off,
}
