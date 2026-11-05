use core::{mem, net::SocketAddr};
use std::collections::HashSet;

use crate::prelude::RejectionReason;

const MAX_NAME_LENGTH: usize = 20;

/// What a registered client takes part as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Player,
    Spectator,
}

#[derive(Debug)]
struct Client {
    name: String,
    addr: SocketAddr,
    role: Role,
}

/// The registered clients and the rules about who may register under which name.
#[derive(Debug, Default)]
pub(crate) struct Registry {
    clients: Vec<Client>,
    replaced: HashSet<SocketAddr>,
}

impl Registry {
    /// Registers a client; a client on the same host registering again replaces its entry.
    pub(crate) fn register(
        &mut self,
        name: &str,
        addr: SocketAddr,
        role: Role,
    ) -> Result<(), RejectionReason> {
        if !is_valid_name(name) {
            return Err(RejectionReason::InvalidName);
        }
        if self
            .clients
            .iter()
            .any(|client| client.name == name && client.addr.ip() != addr.ip())
        {
            return Err(RejectionReason::NameAlreadyTaken);
        }

        let (kept, removed): (Vec<_>, Vec<_>) = mem::take(&mut self.clients)
            .into_iter()
            .partition(|client| client.name != name && client.addr != addr);
        self.clients = kept;
        self.replaced.remove(&addr);
        self.replaced.extend(
            removed
                .into_iter()
                .map(|client| client.addr)
                .filter(|old| *old != addr),
        );
        self.clients.push(Client {
            name: name.to_string(),
            addr,
            role,
        });
        Ok(())
    }

    /// Removes the client at `addr`, even one that a newer registration replaced, and tells
    /// whether there was one.
    pub(crate) fn unregister(&mut self, addr: SocketAddr) -> bool {
        let count = self.clients.len();
        self.clients.retain(|client| client.addr != addr);
        let was_replaced = self.replaced.remove(&addr);
        was_replaced || self.clients.len() != count
    }

    pub(crate) fn name_at(&self, addr: SocketAddr) -> Option<&str> {
        self.clients
            .iter()
            .find(|client| client.addr == addr)
            .map(|client| client.name.as_str())
    }

    pub(crate) fn addr_of(&self, name: &str) -> Option<SocketAddr> {
        self.clients
            .iter()
            .find(|client| client.name == name)
            .map(|client| client.addr)
    }

    pub(crate) fn player_names(&self) -> impl Iterator<Item = &str> {
        self.with_role(Role::Player)
            .map(|client| client.name.as_str())
    }

    pub(crate) fn player_count(&self) -> usize {
        self.with_role(Role::Player).count()
    }

    pub(crate) fn addrs(&self) -> impl Iterator<Item = SocketAddr> {
        self.clients.iter().map(|client| client.addr)
    }

    pub(crate) fn spectator_addrs(&self) -> impl Iterator<Item = SocketAddr> {
        self.with_role(Role::Spectator).map(|client| client.addr)
    }

    fn with_role(&self, role: Role) -> impl Iterator<Item = &Client> {
        self.clients
            .iter()
            .filter(move |client| client.role == role)
    }
}

fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= MAX_NAME_LENGTH
        && !name
            .chars()
            .any(|c| matches!(c, ',' | ';' | ':') || c.is_whitespace())
}

#[cfg(test)]
mod tests {
    use core::net::SocketAddr;

    use super::{Registry, Role};
    use crate::prelude::RejectionReason;

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    fn other_host(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 2], port))
    }

    #[test]
    fn registers_players_and_spectators_separately() {
        let mut registry = Registry::default();
        registry.register("anna", addr(1), Role::Player).unwrap();
        registry.register("zoe", addr(2), Role::Spectator).unwrap();

        assert_eq!(registry.player_names().collect::<Vec<_>>(), ["anna"]);
        assert_eq!(registry.player_count(), 1);
        assert_eq!(registry.spectator_addrs().collect::<Vec<_>>(), [addr(2)]);
        assert_eq!(registry.addrs().count(), 2);
    }

    #[test]
    fn looks_clients_up_by_address_and_name() {
        let mut registry = Registry::default();
        registry.register("anna", addr(1), Role::Player).unwrap();

        assert_eq!(registry.name_at(addr(1)), Some("anna"));
        assert_eq!(registry.name_at(addr(2)), None);
        assert_eq!(registry.addr_of("anna"), Some(addr(1)));
        assert_eq!(registry.addr_of("bert"), None);
    }

    #[test]
    fn a_client_on_the_same_host_takes_over_the_name() {
        let mut registry = Registry::default();
        registry.register("anna", addr(1), Role::Player).unwrap();

        assert_eq!(registry.register("anna", addr(2), Role::Player), Ok(()));
        assert_eq!(registry.addr_of("anna"), Some(addr(2)));
        assert_eq!(registry.player_count(), 1);
    }

    #[test]
    fn a_replaced_client_can_still_unregister_once() {
        let mut registry = Registry::default();
        registry.register("anna", addr(1), Role::Player).unwrap();
        registry.register("anna", addr(2), Role::Player).unwrap();

        assert!(registry.unregister(addr(1)));
        assert!(!registry.unregister(addr(1)));
        assert_eq!(registry.addr_of("anna"), Some(addr(2)));
    }

    #[test]
    fn rejects_a_name_taken_from_another_host() {
        let mut registry = Registry::default();
        registry.register("anna", addr(1), Role::Player).unwrap();

        assert_eq!(
            registry.register("anna", other_host(2), Role::Player),
            Err(RejectionReason::NameAlreadyTaken)
        );
    }

    #[test]
    fn the_same_address_replaces_its_entry() {
        let mut registry = Registry::default();
        registry.register("anna", addr(1), Role::Player).unwrap();
        registry.register("bert", addr(1), Role::Spectator).unwrap();

        assert_eq!(registry.name_at(addr(1)), Some("bert"));
        assert_eq!(registry.addr_of("anna"), None);
        assert_eq!(registry.player_count(), 0);
    }

    #[test]
    fn rejects_invalid_names() {
        let mut registry = Registry::default();

        for name in ["", "a b", "a;b", "a,b", "a:b", &"x".repeat(21)] {
            assert_eq!(
                registry.register(name, addr(1), Role::Player),
                Err(RejectionReason::InvalidName),
                "name {name:?}"
            );
        }
    }

    #[test]
    fn unregistering_tells_whether_a_client_was_removed() {
        let mut registry = Registry::default();
        registry.register("anna", addr(1), Role::Player).unwrap();

        assert!(registry.unregister(addr(1)));
        assert!(!registry.unregister(addr(1)));
    }
}
