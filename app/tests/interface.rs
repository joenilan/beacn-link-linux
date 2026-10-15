use beacn_link_protocol::{catalog, planner::ApplicationId};
use beacn_linx_app::{
    model::{Command, Model, Request, Snapshot},
    ui::{self, Interface},
};
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2};

const ROWS: &str =
    "app\t10\t123\t20\tlive\t6d7076\t6d7076\napp\t11\t124\t21\tlive\t67616d65\t67616d65\n";
fn catalog(generation: u64, rows: &str) -> catalog::Catalog {
    catalog::parse(
        generation,
        &format!("catalog\t42\t99\t70697065776972652d30\n{rows}"),
    )
    .unwrap()
}
fn id(n: u8) -> ApplicationId {
    ApplicationId(format!("pw-client:{}:pid:{}", 10 + n, 123 + n))
}
fn frame(
    ctx: &egui::Context,
    view: &mut Interface,
    snapshot: &Snapshot,
    events: Vec<Event>,
) -> Vec<Command> {
    let mut result = vec![];
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(1060., 760.),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            result = view.show(ctx, snapshot);
        },
    );
    result
}
fn click(
    ctx: &egui::Context,
    view: &mut Interface,
    snapshot: &Snapshot,
    key: &str,
) -> Vec<Command> {
    let center = view.widgets[key].center();
    frame(
        ctx,
        view,
        snapshot,
        vec![
            Event::PointerMoved(center),
            Event::PointerButton {
                pos: center,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
            Event::PointerButton {
                pos: center,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            },
        ],
    )
}
#[test]
fn actual_widgets_select_rename_and_remove_through_existing_selection_owner() {
    let ctx = egui::Context::default();
    ui::style(&ctx);
    let mut view = Interface::default();
    let mut model = Model::default();
    model.observe(catalog(1, ROWS)).unwrap();
    frame(&ctx, &mut view, &model.snapshot(), vec![]);
    let commands = click(
        &ctx,
        &mut view,
        &model.snapshot(),
        &format!("select:{}", id(0).0),
    );
    assert_eq!(commands.len(), 1);
    for command in commands {
        model.command(command).unwrap();
    }
    frame(&ctx, &mut view, &model.snapshot(), vec![]);
    assert_eq!(model.snapshot().selected[&id(0)], "mpv");
    click(
        &ctx,
        &mut view,
        &model.snapshot(),
        &format!("alias:{}", id(0).0),
    );
    frame(
        &ctx,
        &mut view,
        &model.snapshot(),
        vec![
            Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::CTRL,
            },
            Event::Text("Media Player".into()),
        ],
    );
    let commands = click(
        &ctx,
        &mut view,
        &model.snapshot(),
        &format!("save:{}", id(0).0),
    );
    assert_eq!(commands.len(), 1);
    for command in commands {
        model.command(command).unwrap();
    }
    assert_eq!(model.snapshot().selected[&id(0)], "Media Player");
    let commands = click(
        &ctx,
        &mut view,
        &model.snapshot(),
        &format!("remove:{}", id(0).0),
    );
    assert_eq!(commands.len(), 1);
    for command in commands {
        model.command(command).unwrap();
    }
    assert!(model.snapshot().selected.is_empty());
    assert!(click(&ctx, &mut view, &model.snapshot(), "start").is_empty());
}
#[test]
fn search_offline_and_stale_states_do_not_grant_selection_or_device_access() {
    let ctx = egui::Context::default();
    let mut view = Interface::default();
    let mut model = Model::default();
    model.observe(catalog(1, ROWS)).unwrap();
    view.search = "game".into();
    frame(&ctx, &mut view, &model.snapshot(), vec![]);
    assert!(!view.widgets.contains_key(&format!("select:{}", id(0).0)));
    assert!(view.widgets.contains_key(&format!("select:{}", id(1).0)));
    view.search.clear();
    model
        .observe(catalog(2, &ROWS.replace("\tlive\t", "\tstale\t")))
        .unwrap();
    frame(&ctx, &mut view, &model.snapshot(), vec![]);
    assert!(click(
        &ctx,
        &mut view,
        &model.snapshot(),
        &format!("select:{}", id(0).0)
    )
    .is_empty());
    model.disconnected("test disconnect".into());
    frame(&ctx, &mut view, &model.snapshot(), vec![]);
    assert!(click(
        &ctx,
        &mut view,
        &model.snapshot(),
        &format!("select:{}", id(0).0)
    )
    .is_empty());
    assert!(click(&ctx, &mut view, &model.snapshot(), "refresh").is_empty());
}
#[test]
fn duplicate_or_invalid_wire_names_refuse_atomically_and_core_change_revokes_selection() {
    let mut model = Model::default();
    model.observe(catalog(1, ROWS)).unwrap();
    model
        .command(Command::Select(id(0), "Player".into()))
        .unwrap();
    let revision = model.snapshot().revision;
    for name in ["Player", "Café", ""] {
        assert!(model.command(Command::Select(id(1), name.into())).is_err());
        assert_eq!(model.snapshot().revision, revision);
        assert_eq!(model.snapshot().selected.len(), 1);
    }
    let mut replaced = catalog(2, ROWS);
    replaced.core = "different-core".into();
    model.observe(replaced).unwrap();
    assert!(model.snapshot().selected.is_empty());
    model
        .command(Command::Select(id(1), "Game".into()))
        .unwrap();
    model.disconnected("lost core".into());
    assert!(model.snapshot().selected.is_empty());
    assert!(model
        .command(Command::Select(id(1), "Game".into()))
        .is_err());
}
#[test]
fn app_exit_and_reopen_never_select_a_same_named_replacement() {
    let mut model = Model::default();
    model.observe(catalog(1, ROWS)).unwrap();
    model
        .command(Command::Select(id(0), "Player".into()))
        .unwrap();
    model.observe(catalog(2, "")).unwrap();
    assert!(model.snapshot().selected.is_empty());
    model
        .observe(catalog(3, &ROWS.replace("\t123\t", "\t125\t")))
        .unwrap();
    assert!(model.snapshot().selected.is_empty());
}
#[test]
fn queued_ui_action_cannot_select_replaced_core_or_changed_stream_binding() {
    for core_changed in [true, false] {
        let mut model = Model::default();
        model.observe(catalog(1, ROWS)).unwrap();
        let request =
            Request::from_view(Command::Select(id(0), "Player".into()), &model.snapshot()).unwrap();
        let rows = if core_changed {
            ROWS.to_owned()
        } else {
            ROWS.replace("\t20\t", "\t22\t")
        };
        let mut next = catalog(2, &rows);
        if core_changed {
            next.core = "replacement-core".into();
        }
        model.observe(next).unwrap();
        assert!(model.request(request).is_err());
        assert!(model.snapshot().selected.is_empty());
    }
}
