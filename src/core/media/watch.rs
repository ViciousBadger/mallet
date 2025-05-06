use std::sync::mpsc::Receiver;

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use super::{MediaSources, MediaSrc, MediaSrcConf, MediaSync};

pub struct MediaWatcher {
    // We keep the Watcher so it stays alive for as long as it needs to.
    _watcher: Box<dyn Send>,
    notif_recv: Receiver<notify::Result<notify::Event>>,
}

impl MediaWatcher {
    pub fn new(conf: &MediaSrcConf) -> Self {
        use notify::{Event, RecursiveMode, Result, Watcher};

        let (tx, rx) = std::sync::mpsc::channel::<Result<Event>>();

        let mut watcher = notify::recommended_watcher(tx).unwrap();
        watcher
            .watch(conf.fs_base_path.as_path(), RecursiveMode::Recursive)
            .unwrap();

        Self {
            _watcher: Box::new(watcher),
            notif_recv: rx,
        }
    }
}

fn media_watch(
    mut watchers: Local<HashMap<MediaSrc, MediaWatcher>>,
    media_sources: Res<MediaSources>,
    mut commands: Commands,
) {
    // Watch newly added sources.
    for (src, conf) in media_sources.iter() {
        if !watchers.contains_key(src) {
            watchers.insert(*src, MediaWatcher::new(conf));
            info!("Watching {:?}", conf.fs_base_path);
        }
    }

    let mut to_sync: HashSet<MediaSrc> = HashSet::new();

    // Poll wathcers
    for (src, watcher) in watchers.iter() {
        for event in watcher.notif_recv.try_iter() {
            use notify::EventKind;
            match event {
                Ok(event) => {
                    if matches!(
                        event.kind,
                        EventKind::Create(..) | EventKind::Modify(..) | EventKind::Remove(..)
                    ) {
                        to_sync.insert(*src);
                    }
                }
                Err(err) => {
                    dbg!(&err);
                }
            }
        }
    }

    // Sync if changed.
    for src in to_sync {
        // NOTE: Triggering a full sync could be resource intensive
        // when there are a lot of assets. Two things:
        // - media_sync should probably be an async process that collects files via WalkDir
        // and process each file in its own async task.
        // - instead of running a full sync, we could send granular events using the fs notify evs.
        commands.trigger(MediaSync(src));
        info!("Source {:?} changed. Syncing media", src);
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, media_watch);
}
