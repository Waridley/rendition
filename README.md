# Rendition

An online multiplayer arena shooter with 2D pixel art characters in a 3D world.
Rend apart your enemies pixel-by-pixel, literally dis-arming them. Dodge attacks
by turning your paper-thin body sideways. Sneak around by sidling along the
walls.

# Open-source

Rendition is open-source software. The source code is available on
[GitHub](https://github.com/Waridley/rendition). The game is licensed under the
[EUPL-1.2](https://github.com/Waridley/rendition/blob/main/LICENSE) license.

Assets are currently not freely available. This decision may be revisited in the
future. For now, all copyright to files in the `assets` folder is owned by
Sonday Studios, LLC., unless otherwise specified.

# Development

Rendition is built with [Rust](https://www.rust-lang.org/) using the
[Bevy engine](https://bevyengine.org/). The game is currently in early 
development. No contributions are yet accepted while the vision is being
laid out to guide future development.

## Architecture

All Rendition networking code is written in-house on top of pure UDP. While this
may be considered "not-invented-here" syndrome, my personal opinion is that
netcode should always be finely tuned to perform as efficiently as possible for
each specific game. This maximizes the accessibility of the game to players with
different internet connections, minimizes throughput costs, and ensures the
highest level of security and cheat resistance.

### Client-server architecture

Rendition uses a client-server architecture. The server is responsible for
managing the source-of-truth game state and syncing it to clients. The client is
responsible for reading player input, predicting the simulation, sending inputs
to the server, receiving confirmed state from the server, rolling back to that
state, and re-running the simulation multiple times to re-predict the current
state.

#### Simulation

Since multiple versions of the simulation need to perfectly agree, the
simulation code is isolated to the [`rendition-sim`](crates/sim) crate. This
crate contains a Bevy plugin which should not be modified from outside it. It
has to export the `SimSchedule` schedule in order to let the client, server,
and killcam worlds decide exactly *when* to run the simulation, but they should
never modify the schedule itself.

The simulation runs with a fixed delta time value, and does not run
automatically. It must be run by the client, server, and killcam apps, 
potentially multiple times per frame, or not at all when the refresh rate is
higher than delta time. Visual state is interpolated according to the
[Fix your Timestep!](https://gafferongames.com/post/fix_your_timestep/)
technique, but the simulation is what determines the logic of what happens in
the game.

#### Client

The [client app](crates/client-app) is responsible for reading player input, predicting
the simulation, sending inputs to the server, receiving confirmed state from the
server, rolling back to that state, and re-running the simulation multiple times
to re-predict the current state.

It is not exactly what is typically called the "game client" for other games,
which would be the program that only runs on the player's machine and actually
creates a window and renders to it. For rendition, that is the job of the [main
app](src/main.rs), but the main app can also be running in headless mode for
dedicated servers.

#### Server

The [server app](crates/server-app/src/lib.rs) is responsible for managing the
source-of-truth game state and syncing it to clients. It receives input actions
from the network apps for all players, and simulates the world forward. It then
sends the resulting state to all clients for them to synchronize to.

The server app may run on a "host" client, or on a dedicated server. I have some
ideas for how to reduce issues with client-hosted servers, but dedicated server
support is the priority.

#### Synchronization

Latency compensation is based on the architecture employed by Overwatch, as
described in the GDC talk
["Overwatch Gameplay Architecture and Netcode"](https://youtu.be/W3aieHjyNvw?si=LAHbqfxwxrh0ceoy)
by Timothy Ford.

- The server continuously simulates the world at a fixed rate.
- Clients simulate the world slightly ahead of the server, based on their own
  inputs and the last confirmed state from the server.
- If the client receives a confirmed state from the server that is different
  from their predicted state, they roll back to the confirmed state and re-run
  the simulation multiple times to re-predict the current state.
- Missed packets are filled in by assuming player input state did not change,
  and notifying the client that the packet was not received. 
- Unexpected drops in network quality are compensated for by speeding up the
  client simulation rate (without changing the fixed delta time used in the
  simulation), thus increasing the input buffer size on the server.
- A rolling window of input states is sent by the client with each packet to
  reduce the risk that the server completely misses a frame of input.
- Since the game favors the shooter rather than the victim in most cases,
  any combat actions received by the server will be evaluated to see if it's
  even possible that they affected another entity (via bounding box intersection
  tests). If so, those entities need rewound to the perspective of the shooter
  at the time the action was performed to see if they were actually hit.
- If the shooter's ping is extremely high, or if the victim does something to
  mitigate the shot, the victim will end up being favored instead.

#### Killcams

Clients also maintain a previous state of the world for replaying kills from
the perspective of the shooter. When the killcam is active, the client world is
still running, but is not synchronized with the main world, and the [killcam
world](crates/killcam-app) is synchronized instead. The killcam world does not
accept game input, only necessary global actions like "skip killcam", "open
pause menu", etc.

#### Main app

The main app is what coordinates all of the above sub-apps. 

When running the client, It holds the world that actually gets rendered to the
screen. It extracts the simulation state from the active simulation (client or
killcam), and runs any purely visual effects or any other systems not related to
the simulation logic. Bevy's renderer will then extract data from the main world
to be rendered. It also holds the buffer of previous inputs that are getting
sent to the server, and predicted states that might need overridden by the
server. It also handles interpolating visual positions between simulated states
to smooth motion on high refresh rate screens, and to blend misprediction
corrections. 

When running the server app, the main app also holds the buffers of inputs from
clients. 

### Miscellaneous

The game uses a right-handed Z-up coordinate system. This requires some 
annoying minor accommodations for Bevy's default Y-up coordinate system,
but I find it much easier to reason about and more consistent with Blender.

Multiple client-hosted servers might exist for one match as redundancy for the
host disconnecting, as well as to help catch players manipulating the server
behavior. A server might even be run by a player who is not in the match, so
they have no vested interest in the outcome.

