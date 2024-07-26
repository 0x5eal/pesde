use clap::Subcommand;
use pesde::Project;

mod login;
mod logout;
mod whoami;

#[derive(Debug, Subcommand)]
pub enum AuthCommands {
    /// Logs in into GitHub, and stores the token
    Login(login::LoginCommand),
    /// Removes the stored token
    Logout(logout::LogoutCommand),
    /// Prints the username of the currently logged-in user
    #[clap(name = "whoami")]
    WhoAmI(whoami::WhoAmICommand),
}

impl AuthCommands {
    pub fn run(self, project: Project, reqwest: reqwest::blocking::Client) -> anyhow::Result<()> {
        match self {
            AuthCommands::Login(login) => login.run(project, reqwest),
            AuthCommands::Logout(logout) => logout.run(project),
            AuthCommands::WhoAmI(whoami) => whoami.run(project, reqwest),
        }
    }
}
