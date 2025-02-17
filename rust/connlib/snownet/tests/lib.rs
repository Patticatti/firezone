use boringtun::x25519::StaticSecret;
use firezone_relay::{AddressFamily, AllocationPort, ClientSocket, IpStack, PeerSocket};
use ip_packet::*;
use rand::rngs::OsRng;
use snownet::{Answer, Client, ClientNode, Event, Node, RelaySocket, Server, ServerNode, Transmit};
use std::{
    collections::{HashSet, VecDeque},
    iter,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    time::{Duration, Instant, SystemTime},
    vec,
};
use str0m::{net::Protocol, Candidate};
use tracing::{debug_span, Span};
use tracing_subscriber::util::SubscriberInitExt;

#[test]
fn migrate_connection_to_new_relay() {
    let _guard = setup_tracing();
    let mut clock = Clock::new();

    let (alice, bob) = alice_and_bob();

    let mut relays = [(
        1,
        TestRelay::new(
            SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3478),
            debug_span!("Roger"),
        ),
    )];
    let mut alice = TestNode::new(debug_span!("Alice"), alice, "1.1.1.1:80").with_relays(
        "alice",
        HashSet::default(),
        &mut relays,
        clock.now,
    );
    let mut bob = TestNode::new(debug_span!("Bob"), bob, "2.2.2.2:80").with_relays(
        "bob",
        HashSet::default(),
        &mut relays,
        clock.now,
    );
    let firewall = Firewall::default()
        .with_block_rule(&alice, &bob)
        .with_block_rule(&bob, &alice);

    handshake(&mut alice, &mut bob, &clock);

    loop {
        if alice.is_connected_to(&bob) && bob.is_connected_to(&alice) {
            break;
        }

        progress(&mut alice, &mut bob, &mut relays, &firewall, &mut clock);
    }

    // Swap out the relays. "Roger" is being removed (ID 1) and "Robert" is being added (ID 2).
    let mut relays = [(
        2,
        TestRelay::new(
            SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 3478),
            debug_span!("Robert"),
        ),
    )];
    alice = alice.with_relays("alice", HashSet::from([1]), &mut relays, clock.now);
    bob = bob.with_relays("bob", HashSet::from([1]), &mut relays, clock.now);

    // Make some progress. (the fact that we only need 22 clock ticks means we are no relying on timeouts here (22 * 100ms = 2.2s))
    for _ in 0..22 {
        progress(&mut alice, &mut bob, &mut relays, &firewall, &mut clock);
    }

    alice.ping(ip("9.9.9.9"), ip("8.8.8.8"), &bob, clock.now);
    progress(&mut alice, &mut bob, &mut relays, &firewall, &mut clock);
    assert_eq!(bob.packets_from(ip("9.9.9.9")).count(), 1);

    bob.ping(ip("8.8.8.8"), ip("9.9.9.9"), &alice, clock.now);
    progress(&mut alice, &mut bob, &mut relays, &firewall, &mut clock);
    assert_eq!(alice.packets_from(ip("8.8.8.8")).count(), 1);
}

#[test]
fn idle_connection_is_closed_after_5_minutes() {
    let _guard = setup_tracing();
    let mut clock = Clock::new();

    let (alice, bob) = alice_and_bob();

    let mut relays = [(
        1,
        TestRelay::new(
            SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3478),
            debug_span!("Roger"),
        ),
    )];
    let mut alice = TestNode::new(debug_span!("Alice"), alice, "1.1.1.1:80").with_relays(
        "alice",
        HashSet::default(),
        &mut relays,
        clock.now,
    );
    let mut bob = TestNode::new(debug_span!("Bob"), bob, "2.2.2.2:80").with_relays(
        "bob",
        HashSet::default(),
        &mut relays,
        clock.now,
    );
    let firewall = Firewall::default();

    handshake(&mut alice, &mut bob, &clock);

    loop {
        if alice.is_connected_to(&bob) && bob.is_connected_to(&alice) {
            break;
        }

        progress(&mut alice, &mut bob, &mut relays, &firewall, &mut clock);
    }

    alice.ping(ip("9.9.9.9"), ip("8.8.8.8"), &bob, clock.now);
    bob.ping(ip("8.8.8.8"), ip("9.9.9.9"), &alice, clock.now);

    let start = clock.now;

    while clock.elapsed(start) <= Duration::from_secs(5 * 60) {
        progress(&mut alice, &mut bob, &mut relays, &firewall, &mut clock);
    }

    assert_eq!(alice.packets_from(ip("8.8.8.8")).count(), 1);
    assert_eq!(bob.packets_from(ip("9.9.9.9")).count(), 1);
    assert!(alice
        .events
        .contains(&(Event::ConnectionClosed(1), clock.now)));
    assert!(bob
        .events
        .contains(&(Event::ConnectionClosed(1), clock.now)));
}

#[test]
fn connection_times_out_after_20_seconds() {
    let (mut alice, _) = alice_and_bob();

    let created_at = Instant::now();

    let _ = alice.new_connection(1, Instant::now(), created_at);
    alice.handle_timeout(created_at + Duration::from_secs(20));

    assert_eq!(alice.poll_event().unwrap(), Event::ConnectionFailed(1));
}

#[test]
fn connection_without_candidates_times_out_after_10_seconds() {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();

    let start = Instant::now();

    let (mut alice, mut bob) = alice_and_bob();
    let answer = send_offer(&mut alice, &mut bob, start);

    let accepted_at = start + Duration::from_secs(1);
    alice.accept_answer(1, bob.public_key(), answer, accepted_at);

    alice.handle_timeout(accepted_at + Duration::from_secs(10));

    assert_eq!(alice.poll_event().unwrap(), Event::ConnectionFailed(1));
}

#[test]
fn connection_with_candidates_does_not_time_out_after_10_seconds() {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();

    let start = Instant::now();

    let (mut alice, mut bob) = alice_and_bob();
    let answer = send_offer(&mut alice, &mut bob, start);

    let accepted_at = start + Duration::from_secs(1);
    alice.accept_answer(1, bob.public_key(), answer, accepted_at);
    alice.add_local_host_candidate(s("10.0.0.2:4444")).unwrap();
    alice.add_remote_candidate(1, host("10.0.0.1:4444"), accepted_at);

    alice.handle_timeout(accepted_at + Duration::from_secs(10));

    let any_failed =
        iter::from_fn(|| alice.poll_event()).any(|e| matches!(e, Event::ConnectionFailed(_)));

    assert!(!any_failed);
}

#[test]
fn answer_after_stale_connection_does_not_panic() {
    let start = Instant::now();

    let (mut alice, mut bob) = alice_and_bob();
    let answer = send_offer(&mut alice, &mut bob, start);

    let now = start + Duration::from_secs(10);
    alice.handle_timeout(now);

    alice.accept_answer(1, bob.public_key(), answer, now + Duration::from_secs(1));
}

#[test]
fn only_generate_candidate_event_after_answer() {
    let local_candidate = SocketAddr::new(IpAddr::from(Ipv4Addr::LOCALHOST), 10000);

    let mut alice = ClientNode::<u64, u64>::new(StaticSecret::random_from_rng(rand::thread_rng()));
    alice.add_local_host_candidate(local_candidate).unwrap();

    let mut bob = ServerNode::<u64, u64>::new(StaticSecret::random_from_rng(rand::thread_rng()));

    let offer = alice.new_connection(1, Instant::now(), Instant::now());

    assert_eq!(
        alice.poll_event(),
        None,
        "no event to be emitted before accepting the answer"
    );

    let answer = bob.accept_connection(1, offer, alice.public_key(), Instant::now());

    alice.accept_answer(1, bob.public_key(), answer, Instant::now());

    assert!(iter::from_fn(|| alice.poll_event()).any(|ev| ev
        == Event::NewIceCandidate {
            connection: 1,
            candidate: Candidate::host(local_candidate, Protocol::Udp)
                .unwrap()
                .to_sdp_string()
        }));
}

fn setup_tracing() -> tracing::subscriber::DefaultGuard {
    tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter("debug")
        .finish()
        .set_default()
}

fn alice_and_bob() -> (ClientNode<u64, u64>, ServerNode<u64, u64>) {
    let alice = ClientNode::new(StaticSecret::random_from_rng(rand::thread_rng()));
    let bob = ServerNode::new(StaticSecret::random_from_rng(rand::thread_rng()));

    (alice, bob)
}

fn send_offer(
    alice: &mut ClientNode<u64, u64>,
    bob: &mut ServerNode<u64, u64>,
    now: Instant,
) -> Answer {
    let offer = alice.new_connection(1, Instant::now(), now);

    bob.accept_connection(1, offer, alice.public_key(), now)
}

fn host(socket: &str) -> String {
    Candidate::host(s(socket), Protocol::Udp)
        .unwrap()
        .to_sdp_string()
}

fn s(socket: &str) -> SocketAddr {
    socket.parse().unwrap()
}

fn ip(ip: &str) -> IpAddr {
    ip.parse().unwrap()
}

// Heavily inspired by https://github.com/algesten/str0m/blob/7ed5143381cf095f7074689cc254b8c9e50d25c5/src/ice/mod.rs#L547-L647.
struct TestNode<R> {
    node: Node<R, u64, u64>,
    transmits: VecDeque<Transmit<'static>>,

    span: Span,
    received_packets: Vec<IpPacket<'static>>,
    /// The primary interface we use to send packets (e.g. to relays).
    primary: SocketAddr,
    /// All local interfaces.
    local: Vec<SocketAddr>,
    events: Vec<(Event<u64>, Instant)>,

    buffer: Box<[u8; 10_000]>,
}

struct TestRelay {
    inner: firezone_relay::Server<OsRng>,
    listen_addr: RelaySocket,
    span: Span,

    allocations: HashSet<(AddressFamily, AllocationPort)>,
    buffer: Vec<u8>,
}

#[derive(Default)]
struct Firewall {
    blocked: Vec<(SocketAddr, SocketAddr)>,
}

struct Clock {
    start: Instant,
    now: Instant,

    tick_rate: Duration,
    max_time: Instant,
}

impl Clock {
    fn new() -> Self {
        let now = Instant::now();
        let tick_rate = Duration::from_millis(100);
        let one_hour = Duration::from_secs(60) * 60;

        Self {
            start: now,
            now,
            tick_rate,
            max_time: now + one_hour,
        }
    }

    fn tick(&mut self) {
        self.now += self.tick_rate;

        let elapsed = self.elapsed(self.start);

        if elapsed.as_millis() % 60_000 == 0 {
            tracing::info!("Time since start: {elapsed:?}")
        }

        if self.now >= self.max_time {
            panic!("Time exceeded")
        }
    }

    fn elapsed(&self, start: Instant) -> Duration {
        self.now.duration_since(start)
    }
}

impl Firewall {
    fn with_block_rule<R1, R2>(mut self, from: &TestNode<R1>, to: &TestNode<R2>) -> Self {
        self.blocked.push((from.primary, to.primary));

        self
    }
}

impl TestRelay {
    fn new(local: impl Into<RelaySocket>, span: Span) -> Self {
        let local = local.into();
        let inner = firezone_relay::Server::new(to_ip_stack(local), OsRng, 3478, 49152..=65535);

        Self {
            inner,
            listen_addr: local,
            span,
            allocations: HashSet::default(),
            buffer: vec![0u8; (1 << 16) - 1],
        }
    }

    fn wants(&self, dst: SocketAddr) -> bool {
        let is_v4_ctrl = self
            .listen_addr
            .as_v4()
            .is_some_and(|v4| SocketAddr::V4(*v4) == dst);
        let is_v6_ctrl = self
            .listen_addr
            .as_v6()
            .is_some_and(|v6| SocketAddr::V6(*v6) == dst);
        let is_allocation = self.allocations.contains(&match dst {
            SocketAddr::V4(_) => (AddressFamily::V4, AllocationPort::new(dst.port())),
            SocketAddr::V6(_) => (AddressFamily::V6, AllocationPort::new(dst.port())),
        });

        is_v4_ctrl || is_v6_ctrl || is_allocation
    }

    fn matching_listen_socket(&self, other: SocketAddr) -> Option<SocketAddr> {
        match other {
            SocketAddr::V4(_) => Some(SocketAddr::V4(*self.listen_addr.as_v4()?)),
            SocketAddr::V6(_) => Some(SocketAddr::V6(*self.listen_addr.as_v6()?)),
        }
    }

    fn ip4(&self) -> Option<IpAddr> {
        self.listen_addr.as_v4().map(|s| IpAddr::V4(*s.ip()))
    }

    fn ip6(&self) -> Option<IpAddr> {
        self.listen_addr.as_v6().map(|s| IpAddr::V6(*s.ip()))
    }

    fn handle_packet<R>(
        &mut self,
        payload: &[u8],
        sender: SocketAddr,
        dst: SocketAddr,
        other: &mut TestNode<R>,
        now: Instant,
    ) {
        if self.listen_addr.matches(dst) {
            self.handle_client_input(payload, ClientSocket::new(sender), other, now);
            return;
        }

        self.handle_peer_traffic(
            payload,
            PeerSocket::new(sender),
            AllocationPort::new(dst.port()),
            other,
            now,
        )
    }

    fn handle_client_input<R>(
        &mut self,
        payload: &[u8],
        client: ClientSocket,
        receiver: &mut TestNode<R>,
        now: Instant,
    ) {
        if let Some((port, peer)) = self
            .span
            .in_scope(|| self.inner.handle_client_input(payload, client, now))
        {
            let payload = &payload[4..];

            // The `dst` of the relayed packet is what TURN calls a "peer".
            let dst = peer.into_socket();

            // The `src_ip` is the relay's IP
            let src_ip = match dst {
                SocketAddr::V4(_) => {
                    assert!(
                        self.allocations.contains(&(AddressFamily::V4, port)),
                        "IPv4 allocation to be present if we want to send to an IPv4 socket"
                    );

                    self.ip4().expect("listen on IPv4 if we have an allocation")
                }
                SocketAddr::V6(_) => {
                    assert!(
                        self.allocations.contains(&(AddressFamily::V6, port)),
                        "IPv6 allocation to be present if we want to send to an IPv6 socket"
                    );

                    self.ip6().expect("listen on IPv6 if we have an allocation")
                }
            };

            // The `src` of the relayed packet is the relay itself _from_ the allocated port.
            let src = SocketAddr::new(src_ip, port.value());

            // Check if we need to relay to ourselves (from one allocation to another)
            if self.wants(dst) {
                // When relaying to ourselves, we become our own peer.
                let peer_socket = PeerSocket::new(src);
                // The allocation that the data is arriving on is the `dst`'s port.
                let allocation_port = AllocationPort::new(dst.port());

                self.handle_peer_traffic(payload, peer_socket, allocation_port, receiver, now);

                return;
            }

            receiver.receive(dst, src, payload, now);
        }
    }

    fn handle_peer_traffic<R>(
        &mut self,
        payload: &[u8],
        peer: PeerSocket,
        port: AllocationPort,
        receiver: &mut TestNode<R>,
        now: Instant,
    ) {
        if let Some((client, channel)) = self
            .span
            .in_scope(|| self.inner.handle_peer_traffic(payload, peer, port))
        {
            let full_length = firezone_relay::ChannelData::encode_header_to_slice(
                channel,
                payload.len() as u16,
                &mut self.buffer[..4],
            );
            self.buffer[4..full_length].copy_from_slice(payload);

            let receiving_socket = client.into_socket();
            let sending_socket = self.matching_listen_socket(receiving_socket).unwrap();
            receiver.receive(
                receiving_socket,
                sending_socket,
                &self.buffer[..full_length],
                now,
            );
        }
    }

    fn drain_messages<R1, R2>(
        &mut self,
        a1: &mut TestNode<R1>,
        a2: &mut TestNode<R2>,
        now: Instant,
    ) {
        while let Some(command) = self.inner.next_command() {
            match command {
                firezone_relay::Command::SendMessage { payload, recipient } => {
                    let recipient = recipient.into_socket();
                    let sending_socket = self.matching_listen_socket(recipient).unwrap();

                    if a1.local.contains(&recipient) {
                        a1.receive(recipient, sending_socket, &payload, now);
                        continue;
                    }

                    if a2.local.contains(&recipient) {
                        a2.receive(recipient, sending_socket, &payload, now);
                        continue;
                    }

                    panic!("Relay generated traffic for unknown client")
                }
                firezone_relay::Command::CreateAllocation { port, family } => {
                    self.allocations.insert((family, port));
                }
                firezone_relay::Command::FreeAllocation { port, family } => {
                    self.allocations.remove(&(family, port));
                }
            }
        }
    }

    fn make_credentials(&self, username: &str) -> (String, String) {
        let expiry = SystemTime::now() + Duration::from_secs(60);

        let secs = expiry
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("expiry must be later than UNIX_EPOCH")
            .as_secs();

        let password =
            firezone_relay::auth::generate_password(self.inner.auth_secret(), expiry, username);

        (format!("{secs}:{username}"), password)
    }
}

fn to_ip_stack(socket: RelaySocket) -> IpStack {
    match socket {
        RelaySocket::V4(v4) => IpStack::Ip4(*v4.ip()),
        RelaySocket::V6(v6) => IpStack::Ip6(*v6.ip()),
        RelaySocket::Dual { v4, v6 } => IpStack::Dual {
            ip4: *v4.ip(),
            ip6: *v6.ip(),
        },
    }
}

impl<R> TestNode<R> {
    pub fn new(span: Span, node: Node<R, u64, u64>, primary: &str) -> Self {
        let primary = primary.parse().unwrap();

        TestNode {
            node,
            span,
            received_packets: vec![],
            buffer: Box::new([0u8; 10_000]),
            primary,
            local: vec![primary],
            events: Default::default(),
            transmits: Default::default(),
        }
    }

    fn with_relays(
        mut self,
        username: &str,
        to_remove: HashSet<u64>,
        relays: &mut [(u64, TestRelay)],
        now: Instant,
    ) -> Self {
        let turn_servers = relays
            .iter()
            .map(|(idx, relay)| {
                let (username, password) = relay.make_credentials(username);

                (
                    *idx,
                    relay.listen_addr,
                    username,
                    password,
                    "firezone".to_owned(),
                )
            })
            .collect::<HashSet<_>>();

        self.span
            .in_scope(|| self.node.update_relays(to_remove, &turn_servers, now));

        self
    }

    fn is_connected_to<RO>(&self, other: &TestNode<RO>) -> bool {
        self.node.connection_id(other.node.public_key()).is_some()
    }

    fn ping<RO>(&mut self, src: IpAddr, dst: IpAddr, other: &TestNode<RO>, now: Instant) {
        let id = self
            .node
            .connection_id(other.node.public_key())
            .expect("cannot ping not-connected node");

        let transmit = self
            .span
            .in_scope(|| {
                self.node.encapsulate(
                    id,
                    ip_packet::make::icmp_request_packet(src, dst, 1, 0).to_immutable(),
                    now,
                )
            })
            .unwrap()
            .unwrap()
            .into_owned();

        self.transmits.push_back(transmit);
    }

    fn packets_from(&self, src: IpAddr) -> impl Iterator<Item = &IpPacket<'static>> {
        self.received_packets
            .iter()
            .filter(move |p| p.source() == src)
    }

    fn receive(&mut self, local: SocketAddr, from: SocketAddr, packet: &[u8], now: Instant) {
        debug_assert!(self.local.contains(&local));

        if let Some((_, packet)) = self
            .span
            .in_scope(|| {
                self.node
                    .decapsulate(local, from, packet, now, self.buffer.as_mut())
            })
            .unwrap()
        {
            self.received_packets.push(packet.to_immutable().to_owned())
        }
    }

    fn drain_events<RO>(&mut self, other: &mut TestNode<RO>, now: Instant) {
        while let Some(v) = self.span.in_scope(|| self.node.poll_event()) {
            self.events.push((v.clone(), now));

            match v {
                Event::NewIceCandidate {
                    connection,
                    candidate,
                } => other
                    .span
                    .in_scope(|| other.node.add_remote_candidate(connection, candidate, now)),
                Event::InvalidateIceCandidate {
                    connection,
                    candidate,
                } => other
                    .span
                    .in_scope(|| other.node.remove_remote_candidate(connection, candidate)),
                Event::ConnectionEstablished(_)
                | Event::ConnectionFailed(_)
                | Event::ConnectionClosed(_) => {}
            };
        }
    }

    fn drain_transmits<RO>(
        &mut self,
        other: &mut TestNode<RO>,
        relays: &mut [(u64, TestRelay)],
        firewall: &Firewall,
        now: Instant,
    ) {
        for trans in iter::from_fn(|| self.node.poll_transmit()).chain(self.transmits.drain(..)) {
            let payload = &trans.payload;
            let dst = trans.dst;

            if let Some((_, relay)) = relays.iter_mut().find(|(_, r)| r.wants(trans.dst)) {
                relay.handle_packet(payload, self.primary, dst, other, now);
                continue;
            }

            let Some(src) = trans.src else {
                tracing::debug!(target: "router", %dst, "Unknown relay");
                continue;
            };

            // Wasn't traffic for the relay, let's check our firewall.
            if firewall.blocked.contains(&(src, dst)) {
                tracing::debug!(target: "firewall", %src, %dst, "Dropping packet");
                continue;
            }

            if !other.local.contains(&dst) {
                tracing::debug!(target: "router", %src, %dst, "Unknown destination");
                continue;
            }

            // Firewall allowed traffic, let's dispatch it.
            other.receive(dst, src, payload, now);
        }
    }
}

fn handshake(client: &mut TestNode<Client>, server: &mut TestNode<Server>, clock: &Clock) {
    let offer = client
        .span
        .in_scope(|| client.node.new_connection(1, clock.now, clock.now));
    let answer = server.span.in_scope(|| {
        server
            .node
            .accept_connection(1, offer, client.node.public_key(), clock.now)
    });
    client.span.in_scope(|| {
        client
            .node
            .accept_answer(1, server.node.public_key(), answer, clock.now)
    });
}

fn progress<R1, R2>(
    a1: &mut TestNode<R1>,
    a2: &mut TestNode<R2>,
    relays: &mut [(u64, TestRelay)],
    firewall: &Firewall,
    clock: &mut Clock,
) {
    clock.tick();

    a1.drain_events(a2, clock.now);
    a2.drain_events(a1, clock.now);

    a1.drain_transmits(a2, relays, firewall, clock.now);
    a2.drain_transmits(a1, relays, firewall, clock.now);

    for (_, relay) in relays.iter_mut() {
        relay.drain_messages(a1, a2, clock.now);
    }

    if let Some(timeout) = a1.node.poll_timeout() {
        if clock.now >= timeout {
            a1.span.in_scope(|| a1.node.handle_timeout(clock.now));
        }
    }

    if let Some(timeout) = a2.node.poll_timeout() {
        if clock.now >= timeout {
            a2.span.in_scope(|| a2.node.handle_timeout(clock.now));
        }
    }

    for (_, relay) in relays {
        if let Some(timeout) = relay.inner.poll_timeout() {
            if clock.now >= timeout {
                relay
                    .span
                    .in_scope(|| relay.inner.handle_timeout(clock.now))
            }
        }
    }

    a1.drain_events(a2, clock.now);
    a2.drain_events(a1, clock.now);
}
