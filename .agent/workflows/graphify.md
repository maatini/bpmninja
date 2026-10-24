# Workflow: graphify
**Command:** /graphify
**Description:** Turn any folder of files into a navigable knowledge graph

## Steps
Run `graphify update .` for the AST-only refresh (no LLM cost) or `graphify build .` for a full rebuild. A full rebuild only after an explicit request.

If no path argument is given, use `.` (current directory).
