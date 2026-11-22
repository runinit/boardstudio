//! Local command-palette wiring, not an upstream component copy.
//! Owners supply the same callbacks as their controls; no document or tool state lives here.
use dioxus::prelude::*;
use std::{collections::BTreeMap, rc::Rc};

#[derive(Clone, PartialEq)]
pub struct Command {
    pub group: &'static str,
    pub label: String,
    pub reason: Option<String>,
    pub hint: Option<&'static str>,
    pub on_select: EventHandler<()>,
}

type RegisteredCommands = BTreeMap<ScopeId, (Rc<()>, Vec<Command>)>;

#[derive(Clone, Copy)]
pub struct CommandRegistry(Signal<RegisteredCommands>);

pub fn use_command_registry() -> CommandRegistry {
    let registry = CommandRegistry(use_signal(BTreeMap::new));
    use_context_provider(|| registry);
    registry
}

impl CommandRegistry {
    /// Each returned callback admits only the command rendered by this live owner.
    pub fn commands(self) -> Vec<Command> {
        self.0
            .read()
            .iter()
            .flat_map(|(owner, (identity, commands))| {
                commands
                    .iter()
                    .enumerate()
                    .map(|(index, command)| {
                        let owner = *owner;
                        let identity = identity.clone();
                        let rendered = command.clone();
                        Command {
                            on_select: EventHandler::new(move |()| {
                                let callback =
                                    self.0.peek().get(&owner).and_then(|(current, commands)| {
                                        (Rc::ptr_eq(current, &identity)
                                            && commands.get(index) == Some(&rendered)
                                            && rendered.reason.is_none())
                                        .then_some(rendered.on_select)
                                    });
                                if let Some(callback) = callback {
                                    callback.call(());
                                }
                            }),
                            ..command.clone()
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

/// One registration per component scope. Isolated control fixtures need no palette provider.
/// A second hook in the same scope is rejected instead of overwriting sibling commands.
pub fn use_commands(commands: Vec<Command>) {
    let registry = try_use_context::<CommandRegistry>();
    let owner = dioxus::core::current_scope_id();
    let identity = use_hook(|| Rc::new(()));
    use_effect(use_reactive((&commands,), {
        let identity = identity.clone();
        move |(commands,)| {
            let Some(mut registry) = registry else {
                return;
            };
            if let Some((registered, previous)) = registry.0.peek().get(&owner) {
                assert!(
                    Rc::ptr_eq(registered, &identity),
                    "use_commands must be called once per component scope"
                );
                if previous == &commands {
                    return;
                }
            }
            registry
                .0
                .write()
                .insert(owner, (identity.clone(), commands));
        }
    }));
    use_drop(move || {
        if let Some(mut registry) = registry {
            registry.0.write().remove(&owner);
        }
    });
}
