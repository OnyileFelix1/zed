use call::ActiveCall;
use client::Channel;
use gpui::{executor::Deterministic, TestAppContext};
use std::sync::Arc;

use crate::tests::{room_participants, RoomParticipants};

use super::TestServer;

#[gpui::test]
async fn test_basic_channels(
    deterministic: Arc<Deterministic>,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    deterministic.forbid_parking();
    let mut server = TestServer::start(&deterministic).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;

    let channel_a_id = client_a
        .channel_store()
        .update(cx_a, |channel_store, _| {
            channel_store.create_channel("channel-a", None)
        })
        .await
        .unwrap();

    client_a.channel_store().read_with(cx_a, |channels, _| {
        assert_eq!(
            channels.channels(),
            &[Arc::new(Channel {
                id: channel_a_id,
                name: "channel-a".to_string(),
                parent_id: None,
                depth: 0,
            })]
        )
    });

    client_b
        .channel_store()
        .read_with(cx_b, |channels, _| assert_eq!(channels.channels(), &[]));

    // Invite client B to channel A as client A.
    client_a
        .channel_store()
        .update(cx_a, |channel_store, _| {
            channel_store.invite_member(channel_a_id, client_b.user_id().unwrap(), false)
        })
        .await
        .unwrap();

    // Wait for client b to see the invitation
    deterministic.run_until_parked();

    client_b.channel_store().read_with(cx_b, |channels, _| {
        assert_eq!(
            channels.channel_invitations(),
            &[Arc::new(Channel {
                id: channel_a_id,
                name: "channel-a".to_string(),
                parent_id: None,
                depth: 0,
            })]
        )
    });

    // Client B now sees that they are a member channel A.
    client_b
        .channel_store()
        .update(cx_b, |channels, _| {
            channels.respond_to_channel_invite(channel_a_id, true)
        })
        .await
        .unwrap();
    client_b.channel_store().read_with(cx_b, |channels, _| {
        assert_eq!(channels.channel_invitations(), &[]);
        assert_eq!(
            channels.channels(),
            &[Arc::new(Channel {
                id: channel_a_id,
                name: "channel-a".to_string(),
                parent_id: None,
                depth: 0,
            })]
        )
    });

    // Client A deletes the channel
    client_a
        .channel_store()
        .update(cx_a, |channel_store, _| {
            channel_store.remove_channel(channel_a_id)
        })
        .await
        .unwrap();

    deterministic.run_until_parked();
    client_a
        .channel_store()
        .read_with(cx_a, |channels, _| assert_eq!(channels.channels(), &[]));
    client_b
        .channel_store()
        .read_with(cx_b, |channels, _| assert_eq!(channels.channels(), &[]));
}

#[gpui::test]
async fn test_channel_room(
    deterministic: Arc<Deterministic>,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
    cx_c: &mut TestAppContext,
) {
    deterministic.forbid_parking();
    let mut server = TestServer::start(&deterministic).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let client_c = server.create_client(cx_b, "user_c").await;

    let zed_id = server
        .make_channel(
            "zed",
            (&client_a, cx_a),
            &mut [(&client_b, cx_b), (&client_c, cx_c)],
        )
        .await;

    let active_call_a = cx_a.read(ActiveCall::global);
    let active_call_b = cx_b.read(ActiveCall::global);

    active_call_a
        .update(cx_a, |active_call, cx| active_call.join_channel(zed_id, cx))
        .await
        .unwrap();

    // TODO Test that B and C sees A in the channel room
    client_b.channel_store().read_with(cx_b, |channels, _| {
        assert_eq!(
            channels.channels(),
            &[Arc::new(Channel {
                id: zed_id,
                name: "zed".to_string(),
                parent_id: None,
                depth: 0,
            })]
        )
    });

    active_call_b
        .update(cx_b, |active_call, cx| active_call.join_channel(zed_id, cx))
        .await
        .unwrap();

    // TODO Test that C sees A and B in the channel room

    deterministic.run_until_parked();

    let room_a = active_call_a.read_with(cx_a, |call, _| call.room().unwrap().clone());
    room_a.read_with(cx_a, |room, _| assert!(room.is_connected()));
    assert_eq!(
        room_participants(&room_a, cx_a),
        RoomParticipants {
            remote: vec!["user_b".to_string()],
            pending: vec![]
        }
    );

    let room_b = active_call_b.read_with(cx_b, |call, _| call.room().unwrap().clone());
    room_b.read_with(cx_b, |room, _| assert!(room.is_connected()));
    assert_eq!(
        room_participants(&room_b, cx_b),
        RoomParticipants {
            remote: vec!["user_a".to_string()],
            pending: vec![]
        }
    );

    // Make sure that leaving and rejoining works

    active_call_a
        .update(cx_a, |active_call, cx| active_call.hang_up(cx))
        .await
        .unwrap();

    // TODO Make sure that C sees A leave

    active_call_b
        .update(cx_b, |active_call, cx| active_call.hang_up(cx))
        .await
        .unwrap();

    // TODO Make sure that C sees B leave

    active_call_a
        .update(cx_a, |active_call, cx| active_call.join_channel(zed_id, cx))
        .await
        .unwrap();

    active_call_b
        .update(cx_b, |active_call, cx| active_call.join_channel(zed_id, cx))
        .await
        .unwrap();

    deterministic.run_until_parked();

    let room_a = active_call_a.read_with(cx_a, |call, _| call.room().unwrap().clone());
    room_a.read_with(cx_a, |room, _| assert!(room.is_connected()));
    assert_eq!(
        room_participants(&room_a, cx_a),
        RoomParticipants {
            remote: vec!["user_b".to_string()],
            pending: vec![]
        }
    );

    let room_b = active_call_b.read_with(cx_b, |call, _| call.room().unwrap().clone());
    room_b.read_with(cx_b, |room, _| assert!(room.is_connected()));
    assert_eq!(
        room_participants(&room_b, cx_b),
        RoomParticipants {
            remote: vec!["user_a".to_string()],
            pending: vec![]
        }
    );
}