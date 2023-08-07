use crate::tests::{room_participants, RoomParticipants, TestServer};
use call::ActiveCall;
use client::{Channel, ChannelMembership, User};
use gpui::{executor::Deterministic, TestAppContext};
use rpc::proto;
use std::sync::Arc;

#[gpui::test]
async fn test_core_channels(
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
    let channel_b_id = client_a
        .channel_store()
        .update(cx_a, |channel_store, _| {
            channel_store.create_channel("channel-b", Some(channel_a_id))
        })
        .await
        .unwrap();

    deterministic.run_until_parked();
    client_a.channel_store().read_with(cx_a, |channels, _| {
        assert_eq!(
            channels.channels(),
            &[
                Arc::new(Channel {
                    id: channel_a_id,
                    name: "channel-a".to_string(),
                    parent_id: None,
                    user_is_admin: true,
                    depth: 0,
                }),
                Arc::new(Channel {
                    id: channel_b_id,
                    name: "channel-b".to_string(),
                    parent_id: Some(channel_a_id),
                    user_is_admin: true,
                    depth: 1,
                })
            ]
        )
    });

    client_b
        .channel_store()
        .read_with(cx_b, |channels, _| assert_eq!(channels.channels(), &[]));

    // Invite client B to channel A as client A.
    client_a
        .channel_store()
        .update(cx_a, |store, cx| {
            assert!(!store.has_pending_channel_invite(channel_a_id, client_b.user_id().unwrap()));

            let invite = store.invite_member(channel_a_id, client_b.user_id().unwrap(), false, cx);

            // Make sure we're synchronously storing the pending invite
            assert!(store.has_pending_channel_invite(channel_a_id, client_b.user_id().unwrap()));
            invite
        })
        .await
        .unwrap();

    // Client A sees that B has been invited.
    deterministic.run_until_parked();
    client_b.channel_store().read_with(cx_b, |channels, _| {
        assert_eq!(
            channels.channel_invitations(),
            &[Arc::new(Channel {
                id: channel_a_id,
                name: "channel-a".to_string(),
                parent_id: None,
                user_is_admin: false,
                depth: 0,
            })]
        )
    });
    let members = client_a
        .channel_store()
        .update(cx_a, |store, cx| {
            assert!(!store.has_pending_channel_invite(channel_a_id, client_b.user_id().unwrap()));
            store.get_channel_member_details(channel_a_id, cx)
        })
        .await
        .unwrap();
    assert_members_eq(
        &members,
        &[
            (
                client_a.user_id().unwrap(),
                true,
                proto::channel_member::Kind::Member,
            ),
            (
                client_b.user_id().unwrap(),
                false,
                proto::channel_member::Kind::Invitee,
            ),
        ],
    );

    // Client B accepts the invitation.
    client_b
        .channel_store()
        .update(cx_b, |channels, _| {
            channels.respond_to_channel_invite(channel_a_id, true)
        })
        .await
        .unwrap();
    deterministic.run_until_parked();

    // Client B now sees that they are a member of channel A and its existing subchannels.
    client_b.channel_store().read_with(cx_b, |channels, _| {
        assert_eq!(channels.channel_invitations(), &[]);
        assert_eq!(
            channels.channels(),
            &[
                Arc::new(Channel {
                    id: channel_a_id,
                    name: "channel-a".to_string(),
                    parent_id: None,
                    user_is_admin: false,
                    depth: 0,
                }),
                Arc::new(Channel {
                    id: channel_b_id,
                    name: "channel-b".to_string(),
                    parent_id: Some(channel_a_id),
                    user_is_admin: false,
                    depth: 1,
                })
            ]
        )
    });

    let channel_c_id = client_a
        .channel_store()
        .update(cx_a, |channel_store, _| {
            channel_store.create_channel("channel-c", Some(channel_b_id))
        })
        .await
        .unwrap();

    deterministic.run_until_parked();
    client_b.channel_store().read_with(cx_b, |channels, _| {
        assert_eq!(
            channels.channels(),
            &[
                Arc::new(Channel {
                    id: channel_a_id,
                    name: "channel-a".to_string(),
                    parent_id: None,
                    user_is_admin: false,
                    depth: 0,
                }),
                Arc::new(Channel {
                    id: channel_b_id,
                    name: "channel-b".to_string(),
                    parent_id: Some(channel_a_id),
                    user_is_admin: false,
                    depth: 1,
                }),
                Arc::new(Channel {
                    id: channel_c_id,
                    name: "channel-c".to_string(),
                    parent_id: Some(channel_b_id),
                    user_is_admin: false,
                    depth: 2,
                }),
            ]
        )
    });

    // Update client B's membership to channel A to be an admin.
    client_a
        .channel_store()
        .update(cx_a, |store, cx| {
            store.set_member_admin(channel_a_id, client_b.user_id().unwrap(), true, cx)
        })
        .await
        .unwrap();
    deterministic.run_until_parked();

    // Observe that client B is now an admin of channel A, and that
    // their admin priveleges extend to subchannels of channel A.
    client_b.channel_store().read_with(cx_b, |channels, _| {
        assert_eq!(channels.channel_invitations(), &[]);
        assert_eq!(
            channels.channels(),
            &[
                Arc::new(Channel {
                    id: channel_a_id,
                    name: "channel-a".to_string(),
                    parent_id: None,
                    user_is_admin: true,
                    depth: 0,
                }),
                Arc::new(Channel {
                    id: channel_b_id,
                    name: "channel-b".to_string(),
                    parent_id: Some(channel_a_id),
                    user_is_admin: false,
                    depth: 1,
                }),
                Arc::new(Channel {
                    id: channel_c_id,
                    name: "channel-c".to_string(),
                    parent_id: Some(channel_b_id),
                    user_is_admin: false,
                    depth: 2,
                }),
            ]
        );

        assert!(channels.is_user_admin(channel_c_id))
    });

    // Client A deletes the channel, deletion also deletes subchannels.
    client_a
        .channel_store()
        .update(cx_a, |channel_store, _| {
            channel_store.remove_channel(channel_b_id)
        })
        .await
        .unwrap();

    deterministic.run_until_parked();
    client_a.channel_store().read_with(cx_a, |channels, _| {
        assert_eq!(
            channels.channels(),
            &[Arc::new(Channel {
                id: channel_a_id,
                name: "channel-a".to_string(),
                parent_id: None,
                user_is_admin: true,
                depth: 0,
            })]
        )
    });
    client_b.channel_store().read_with(cx_b, |channels, _| {
        assert_eq!(
            channels.channels(),
            &[Arc::new(Channel {
                id: channel_a_id,
                name: "channel-a".to_string(),
                parent_id: None,
                user_is_admin: true,
                depth: 0,
            })]
        )
    });

    // Remove client B
    client_a
        .channel_store()
        .update(cx_a, |channel_store, cx| {
            channel_store.remove_member(channel_a_id, client_b.user_id().unwrap(), cx)
        })
        .await
        .unwrap();

    deterministic.run_until_parked();

    // Client A still has their channel
    client_a.channel_store().read_with(cx_a, |channels, _| {
        assert_eq!(
            channels.channels(),
            &[Arc::new(Channel {
                id: channel_a_id,
                name: "channel-a".to_string(),
                parent_id: None,
                user_is_admin: true,
                depth: 0,
            })]
        )
    });

    // Client B is gone
    client_b
        .channel_store()
        .read_with(cx_b, |channels, _| assert_eq!(channels.channels(), &[]));
}

fn assert_participants_eq(participants: &[Arc<User>], expected_partitipants: &[u64]) {
    assert_eq!(
        participants.iter().map(|p| p.id).collect::<Vec<_>>(),
        expected_partitipants
    );
}

fn assert_members_eq(
    members: &[ChannelMembership],
    expected_members: &[(u64, bool, proto::channel_member::Kind)],
) {
    assert_eq!(
        members
            .iter()
            .map(|member| (member.user.id, member.admin, member.kind))
            .collect::<Vec<_>>(),
        expected_members
    );
}

#[gpui::test]
async fn test_joining_channel_ancestor_member(
    deterministic: Arc<Deterministic>,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    deterministic.forbid_parking();
    let mut server = TestServer::start(&deterministic).await;

    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;

    let parent_id = server
        .make_channel("parent", (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    let sub_id = client_a
        .channel_store()
        .update(cx_a, |channel_store, _| {
            channel_store.create_channel("sub_channel", Some(parent_id))
        })
        .await
        .unwrap();

    let active_call_b = cx_b.read(ActiveCall::global);

    assert!(active_call_b
        .update(cx_b, |active_call, cx| active_call.join_channel(sub_id, cx))
        .await
        .is_ok());
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
    let client_c = server.create_client(cx_c, "user_c").await;

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

    // Give everyone a chance to observe user A joining
    deterministic.run_until_parked();

    client_a.channel_store().read_with(cx_a, |channels, _| {
        assert_participants_eq(
            channels.channel_participants(zed_id),
            &[client_a.user_id().unwrap()],
        );
    });

    client_b.channel_store().read_with(cx_b, |channels, _| {
        assert_participants_eq(
            channels.channel_participants(zed_id),
            &[client_a.user_id().unwrap()],
        );
        assert_eq!(
            channels.channels(),
            &[Arc::new(Channel {
                id: zed_id,
                name: "zed".to_string(),
                parent_id: None,
                user_is_admin: false,
                depth: 0,
            })]
        )
    });

    client_c.channel_store().read_with(cx_c, |channels, _| {
        assert_participants_eq(
            channels.channel_participants(zed_id),
            &[client_a.user_id().unwrap()],
        );
    });

    active_call_b
        .update(cx_b, |active_call, cx| active_call.join_channel(zed_id, cx))
        .await
        .unwrap();

    deterministic.run_until_parked();

    client_a.channel_store().read_with(cx_a, |channels, _| {
        assert_participants_eq(
            channels.channel_participants(zed_id),
            &[client_a.user_id().unwrap(), client_b.user_id().unwrap()],
        );
    });

    client_b.channel_store().read_with(cx_b, |channels, _| {
        assert_participants_eq(
            channels.channel_participants(zed_id),
            &[client_a.user_id().unwrap(), client_b.user_id().unwrap()],
        );
    });

    client_c.channel_store().read_with(cx_c, |channels, _| {
        assert_participants_eq(
            channels.channel_participants(zed_id),
            &[client_a.user_id().unwrap(), client_b.user_id().unwrap()],
        );
    });

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

    deterministic.run_until_parked();

    client_a.channel_store().read_with(cx_a, |channels, _| {
        assert_participants_eq(
            channels.channel_participants(zed_id),
            &[client_b.user_id().unwrap()],
        );
    });

    client_b.channel_store().read_with(cx_b, |channels, _| {
        assert_participants_eq(
            channels.channel_participants(zed_id),
            &[client_b.user_id().unwrap()],
        );
    });

    client_c.channel_store().read_with(cx_c, |channels, _| {
        assert_participants_eq(
            channels.channel_participants(zed_id),
            &[client_b.user_id().unwrap()],
        );
    });

    active_call_b
        .update(cx_b, |active_call, cx| active_call.hang_up(cx))
        .await
        .unwrap();

    deterministic.run_until_parked();

    client_a.channel_store().read_with(cx_a, |channels, _| {
        assert_participants_eq(channels.channel_participants(zed_id), &[]);
    });

    client_b.channel_store().read_with(cx_b, |channels, _| {
        assert_participants_eq(channels.channel_participants(zed_id), &[]);
    });

    client_c.channel_store().read_with(cx_c, |channels, _| {
        assert_participants_eq(channels.channel_participants(zed_id), &[]);
    });

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

#[gpui::test]
async fn test_channel_jumping(deterministic: Arc<Deterministic>, cx_a: &mut TestAppContext) {
    deterministic.forbid_parking();
    let mut server = TestServer::start(&deterministic).await;
    let client_a = server.create_client(cx_a, "user_a").await;

    let zed_id = server.make_channel("zed", (&client_a, cx_a), &mut []).await;
    let rust_id = server
        .make_channel("rust", (&client_a, cx_a), &mut [])
        .await;

    let active_call_a = cx_a.read(ActiveCall::global);

    active_call_a
        .update(cx_a, |active_call, cx| active_call.join_channel(zed_id, cx))
        .await
        .unwrap();

    // Give everything a chance to observe user A joining
    deterministic.run_until_parked();

    client_a.channel_store().read_with(cx_a, |channels, _| {
        assert_participants_eq(
            channels.channel_participants(zed_id),
            &[client_a.user_id().unwrap()],
        );
        assert_participants_eq(channels.channel_participants(rust_id), &[]);
    });

    active_call_a
        .update(cx_a, |active_call, cx| {
            active_call.join_channel(rust_id, cx)
        })
        .await
        .unwrap();

    deterministic.run_until_parked();

    client_a.channel_store().read_with(cx_a, |channels, _| {
        assert_participants_eq(channels.channel_participants(zed_id), &[]);
        assert_participants_eq(
            channels.channel_participants(rust_id),
            &[client_a.user_id().unwrap()],
        );
    });
}