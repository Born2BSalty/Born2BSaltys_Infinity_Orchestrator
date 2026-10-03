# Gallery feed

This folder is the source of the curated modlist gallery BIO shows on the Install page. It is laid out so it could be lifted into its own repository unchanged.

## Layout

```
gallery/
  README.md
  <id>/
    entry.json           maintainer metadata
    modlist.biolist       the modlist file, exported from BIO
    cover.png             optional card art
```

The BIO build embeds every folder's `entry.json`, `modlist.biolist` and `cover.png` into the exe, and assembles the gallery from them at startup. A change here ships with the next release.

## `entry.json` fields

| Field | Meaning | Rule |
|---|---|---|
| `id` | folder name, the entry's permanent key | lowercase letters, digits, dashes; must equal the folder name; unique |
| `name` | card title | 1 to 80 characters |
| `author` | handle shown as "by ..." | 1 to 80 characters |
| `description` | card text and the Details page description | 1 to 500 characters |
| `tags` | pills on the card, searchable | up to 6, each up to 20 characters |
| `game` | `BGEE`, `BG2EE`, `IWDEE` or `EET` | must match the game inside the modlist file |
| `featured` | shows first among entries with the same `order`, and passes the "Featured only" filter | true or false |
| `version` | the list's own version, shown on Details | free text, 1 to 20 characters |
| `requirements` | optional; the "Requires" fact on Details | when absent, the game's default sentence is used |
| `order` | optional; the card's position in the gallery | a whole number, default 0; lower comes first; entries with the same order show featured first, then by name |

## Workflow

Drop a folder (`entry.json`, `modlist.biolist`, optional `cover.png`) into `gallery/`, commit, then build BIO.

```
cargo run --release --bin gallery-index -- check gallery
```

`check` validates every folder; it is the same check CI runs on every pull request that touches `gallery/`.

Submissions arrive on the BIO Discord; the maintainer places the file here.
