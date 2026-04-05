import os

def fix_tests(path):
    with open(path, "r") as f:
        content = f.read()

    # Replace `.values()` with `.iter().map(|r| r.value().clone())`
    content = content.replace("engine.pending_service_tasks.values()", "engine.pending_service_tasks.iter().map(|r| r.value().clone())")
    content = content.replace("engine.pending_user_tasks.values()", "engine.pending_user_tasks.iter().map(|r| r.value().clone())")
    content = content.replace("engine.pending_timers.values()", "engine.pending_timers.iter().map(|r| r.value().clone())")
    content = content.replace("engine.pending_message_catches.values()", "engine.pending_message_catches.iter().map(|r| r.value().clone())")
    content = content.replace("msgs.values()", "msgs.iter().map(|r| r.value().clone())")

    with open(path, "w") as f:
        f.write(content)

fix_tests("src/engine/tests.rs")
