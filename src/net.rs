use sim::ClientId;
use sim::players::PlayerId;
use bevy::prelude::*;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

const DEFAULT_SERVER_PORT: u16 = 6464;
const DEFAULT_CLIENT_PORT: u16 = 6465;

/// Handles all raw network communication.
///
/// The main app should gather input from the client app and put it into a queue, which is then sent
/// by this plugin to the server, and the main app should gather confirmed state from the server and
/// put it into a queue, which is then sent by this plugin to relevant clients.
pub struct NetPlugin;

impl Plugin for NetPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<RemoteAddr<ToServer>>()
			.register_type::<RemoteAddr<ToClient>>();
	}
}

/// A remote connection to another app. In the server, this is a connection to a client. In the
/// client, this is a connection to a server. See [`ToServer`] and [`ToClient`].
///
/// The "remote" might actually be the same machine (localhost) if the server is running on a "host"
/// client.
///
/// Multiple servers might exist for one match, but only one is the authority. The others are
/// fallbacks in case the authority disconnects. However, the others may also be compared to the
/// authority to detect cheating or bugs.

// I think the raw connections should be private, and apps should only communicate through the
// NetPlugin.
#[derive(Reflect, Component)]
#[reflect(Component)]
struct RemoteAddr<To: Remote> {
	address: SocketAddr,
	info: To,
}

impl Default for RemoteAddr<ToServer> {
	fn default() -> Self {
		Self {
			address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, DEFAULT_SERVER_PORT)),
			info: ToServer {},
		}
	}
}

impl Default for RemoteAddr<ToClient> {
	fn default() -> Self {
		Self {
			address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, DEFAULT_CLIENT_PORT)),
			info: ToClient {
				client_id: ClientId::Player(PlayerId(0)),
			},
		}
	}
}

/// Trait for the kind of remote a [`RemoteAddr`] is made to.
#[reflect_trait]
trait Remote {}

/// A remote connection from a client to a server (which may be the same machine).
#[derive(Reflect, Debug, Clone)]
#[reflect(Remote)]
struct ToServer {}
impl Remote for ToServer {}

/// A remote connection from a server to a client (which may be the same machine).
#[derive(Reflect, Debug, Clone)]
#[reflect(Remote)]
struct ToClient {
	client_id: ClientId,
}
impl Remote for ToClient {}
