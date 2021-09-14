use ovh::client::OvhClient;
use ovh::email_redir::OvhMailRedir;

use clap::Clap;

/// A simple CLI tool to handle email redirections using OVH's REST API
#[derive(Clap)]
struct Opts {
    /// File containing API credentials
    #[clap(short, long, default_value = "ovh.conf")]
    config: String,

    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
struct ListArgs {
    /// Domain to list the aliases from
    domain: String,
}

#[derive(Clap)]
struct CreateArgs {
    /// Domain to create the alias to
    domain: String,

    /// Address to create an alias from
    from: String,

    /// Address to forward the emails to
    to: String,

    #[clap(short, long)]
    /// Keep local copy of redirected messages
    local_copy: bool,
}

#[derive(Clap)]
struct DeleteArgs {
    domain: String,

    id: String,
}

#[derive(Clap)]
enum SubCommand {
    /// List all redirections for a given domain
    List(ListArgs),

    /// Create a redirection
    Create(CreateArgs),

    /// Delete a redirection
    Delete(DeleteArgs),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    let c = OvhClient::from_conf(&opts.config)?;

    match opts.subcmd {
        SubCommand::List(a) => {
            let resp = OvhMailRedir::list_redirs(&c, &a.domain).await?;
            for redir in resp {
                println!("{}", redir);
            }
        }
        SubCommand::Create(a) => {
            let resp = OvhMailRedir::create(&c, &a.domain, &a.from, &a.to, a.local_copy).await?;
            println!("{:#?}", resp);
            println!("{:#?}", resp.text().await?);
        }
        SubCommand::Delete(a) => {
            let resp = OvhMailRedir::delete(&c, &a.domain, &a.id).await?;
            println!("{:#?}", resp);
        }
    }

    Ok(())
}
