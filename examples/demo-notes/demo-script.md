# Demo Script

Use these commands when recording the project video:

```bash
cargo run -- search ownership --root ./examples/demo-notes
cargo run -- links --root ./examples/demo-notes
cargo run -- suggest-links --root ./examples/demo-notes --limit 6
cargo run -- doctor --root ./examples/demo-notes
cargo run -- report --root ./examples/demo-notes --format html --output demo-report.html
```

The sample notes intentionally contain one broken link, several missing-but-obvious relationships, and a few orphan documents.

#demo #script
