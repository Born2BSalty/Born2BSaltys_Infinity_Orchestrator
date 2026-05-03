# Mod Downloads: Short Beginner Guide

BIO uses **mod download sources** to know where mods can be downloaded from.

Think of it like this:

```text
mod = the thing you want
source = where BIO can download it

## The Two Files

### Default source list

default_mod_downloads.toml

This is BIO’s built-in list.
Do not edit the AppData copy because BIO may replace it during updates.

### User source list

mod_downloads_user.toml

This is your personal file.
Use this for custom sources, forks, fixes, and missing mods.

BIO has buttons in Step 2 to help edit this file.

## Basic Mod Block

[[mods]]
name = "EE Fixpack"
tp2 = "eefixpack"

Meaning:

name = display name
tp2  = the WeiDU setup name

If the file is:

Setup-XGTCumulativeCasterLevels.tp2

use:

tp2 = "XGTCumulativeCasterLevels"

Do not include Setup- or .tp2.

## Basic Source Block

[[mods.sources]]
id = "gibberlings3"
label = "Gibberlings3"
type = "github"
url = "https://github.com/gibberlings3/ee_fixpack"
repo = "gibberlings3/ee_fixpack"

Meaning:

id    = short internal source name
label = name shown in BIO dropdown
type  = source type
url   = page to open/check
repo  = GitHub owner/repo

A mod can have multiple sources.
First source is normally the main one. Others are forks/backups.

## Common Source Types

type = "github"

BIO checks a GitHub repo.

type = "url"

BIO uses a direct archive/link.

type = "page"

BIO checks a known download page, like Weaselmods or Morpheus-Mart.

## GitHub Release Example

[[mods]]
name = "Example Mod"
tp2 = "examplemod"

[[mods.sources]]
id = "mainauthor"
label = "MainAuthor"
type = "github"
url = "https://github.com/MainAuthor/ExampleMod"
repo = "MainAuthor/ExampleMod"
pkg_windows = "wzp,zip"
pkg_linux = "lin,zip"
pkg_macos = "mac,zip"

Use this when the repo has GitHub Releases.

## GitHub Branch Example

[[mods]]
name = "Example Mod"
tp2 = "examplemod"

[[mods.sources]]
id = "forkauthor"
label = "ForkAuthor"
type = "github"
url = "https://github.com/ForkAuthor/ExampleMod"
repo = "ForkAuthor/ExampleMod"
branch = "master"

Use this when the repo has no releases and BIO should use a branch snapshot.

## Package Picking

pkg_windows = "wzp,zip"
pkg_linux = "lin,zip"
pkg_macos = "mac,zip"

BIO checks these words from left to right.

wzp = Windows zip package
lin = Linux package
mac = macOS package
zip = normal zip fallback

So:

Windows: try Windows zip, then any zip
Linux:   try Linux package, then any zip
macOS:   try macOS package, then any zip



## Source Dropdown In Step 2

In Step 2 → Check Updates:

Mod | Dropdown | Edit Source | Add Source | Open Source

The dropdown chooses which source BIO should use.

Example:

EEFIXPACK → CamDawg

Means:

Check/download EE Fixpack from CamDawg.

It does not mean CamDawg is installed.

## Installed Source vs Update Source

Installed Source = what BIO actually extracted
Update Source    = what BIO will check/download from

If BIO did not extract the mod itself:

Installed Source: Unknown

BIO does not guess from folders or WeiDU logs.

## Adding A Missing Mod

Use:

Step 2 → Check Updates → Add Source

Then add something like:

[[mods]]
name = "My Cool Mod"
tp2 = "mycoolmod"

[[mods.sources]]
id = "authorname"
label = "AuthorName"
type = "github"
url = "https://github.com/AuthorName/MyCoolMod"
repo = "AuthorName/MyCoolMod"
branch = "main"

Then click:

Reload Sources
Check Updates

## Safe Rule

Edit this:

mod_downloads_user.toml

Do not manually edit the AppData copy of:

mod_downloads_default.toml

BIO can replace the default file when updating.