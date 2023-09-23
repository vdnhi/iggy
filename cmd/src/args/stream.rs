use crate::args::common::ListMode;
use clap::{Args, Subcommand};
use iggy::identifier::Identifier;

#[derive(Debug, Subcommand)]
pub(crate) enum StreamAction {
    /// Create stream with given ID and name
    Create(StreamCreateArgs),
    /// Delete stream with given ID
    Delete(StreamDeleteArgs),
    /// Update stream name for given stream ID
    Update(StreamUpdateArgs),
    /// Get details of a single stream with given ID
    Get(StreamGetArgs),
    /// List all streams
    List(StreamListArgs),
}

#[derive(Debug, Args)]
pub(crate) struct StreamCreateArgs {
    /// Stream ID to create topic
    pub(crate) stream_id: u32,
    /// Name of the stream
    pub(crate) name: String,
}

#[derive(Debug, Args)]
pub(crate) struct StreamDeleteArgs {
    /// Stream ID to delete
    ///
    /// Stream ID can be specified as a stream name or ID
    pub(crate) stream_id: Identifier,
}

#[derive(Debug, Args)]
pub(crate) struct StreamUpdateArgs {
    /// Stream ID to update
    ///
    /// Stream ID can be specified as a stream name or ID
    pub(crate) stream_id: Identifier,
    /// New name for the stream
    pub(crate) name: String,
}

#[derive(Debug, Args)]
pub(crate) struct StreamGetArgs {
    /// Stream ID to get
    ///
    /// Stream ID can be specified as a stream name or ID
    pub(crate) stream_id: Identifier,
}

#[derive(Debug, Args)]
pub(crate) struct StreamListArgs {
    /// List mode (table or list)
    #[clap(short, long, value_enum, default_value_t = ListMode::Table)]
    pub(crate) list_mode: ListMode,
}