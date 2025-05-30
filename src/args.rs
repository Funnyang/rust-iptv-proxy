use argh::FromArgs;

#[derive(FromArgs, Clone)]
pub(crate) struct Args {

    #[argh(option, short = 'b', default = r#"String::from("0.0.0.0:7878")"#)]
    pub(crate) bind: String,

    #[argh(option, short = 'I')]
    pub(crate) interface: Option<String>,
}
