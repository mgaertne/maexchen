use core::net::SocketAddr;

use super::{TimerId, table::Table};
use crate::prelude::ClientCommand;

/// The phase a game is in; it reacts to timers and player commands and names the next phase.
///
/// A phase that is not the one a timer or command was meant for returns itself unchanged, which
/// is how late answers and late timeouts are discarded.
pub(super) trait State<D>: Send + Sync {
    /// A timer has run out.
    fn on_timer(self: Box<Self>, table: &mut Table<D>, id: TimerId) -> Next<D>;

    /// The registered player `name` sent a command.
    fn on_player_command(
        self: Box<Self>,
        table: &mut Table<D>,
        name: &str,
        command: ClientCommand,
    ) -> Next<D>;

    fn is_idle(&self) -> bool {
        false
    }

    /// The name of the player of the running round that was last seen at `addr`.
    fn player_at(&self, _addr: SocketAddr) -> Option<String> {
        None
    }

    /// The client at `addr` has left; the running round keeps its player but stops sending.
    fn forget(&mut self, _addr: SocketAddr) {}
}

pub(super) type Next<D> = Box<dyn State<D>>;
