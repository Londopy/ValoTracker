# Disclaimer & Legal Notice

## Use At Your Own Risk

`ValoTracker` is an independent, open-source tool created for personal use and educational
purposes. By downloading, installing, or running this software, you agree to the
following terms in full.

---

## No Affiliation With Riot Games

`ValoTracker` is not affiliated with, endorsed by, sponsored by, or in any way officially
connected to Riot Games, Inc. "VALORANT," "Riot Games," and all related names,
marks, and logos are trademarks of Riot Games, Inc.

---

## How ValoTracker Works (and Why It Should Be Fine)

`ValoTracker` works exclusively by reading data from **VALORANT's own local client API** —
a set of HTTP endpoints that Riot exposes on `127.0.0.1` (localhost) while the
game is running. These endpoints are:

- **Intended for use by Riot's own client software** and are documented and
  used by many community tools
- **Read-only** — `ValoTracker` never writes to, modifies, or injects into the game
  process, game files, or any Riot service
- **Credential-free** — `ValoTracker` reads an authentication token that Riot's own
  client writes to your local machine; it does not ask for your username or
  password, and it does not transmit your credentials anywhere
- **Local-only** — all data processing happens on your machine; `ValoTracker` makes
  outbound requests only to Riot's own PD/GLZ API endpoints (the same ones
  the official client uses) to resolve player names and ranks

`ValoTracker` does **not**:
- Inject code into the VALORANT process
- Read game memory
- Modify any game files or network packets
- Use any exploit, cheat, or bypass
- Interact with Vanguard (Riot's anti-cheat) in any way

This places `ValoTracker` in the same category as many other community tools (Blitz,
Tracker.gg overlay, etc.) that read from the same local endpoints.

---

## No Guarantee of Safety

Despite the above, **Riot Games reserves the right to update their Terms of
Service, ban policy, or anti-cheat systems at any time and without notice.**

The author(s) of `ValoTracker`:

- Make **no guarantee** that using this tool will not result in a ban,
  suspension, or other penalty to your VALORANT account
- Accept **no responsibility** for any action Riot Games takes against your
  account as a result of using this software
- Accept **no responsibility** for any damage to your computer, data, or
  accounts arising from the use of this software
- Provide this software **as-is**, with no warranty of any kind, express or
  implied

**You use this tool entirely at your own risk. If you are concerned about
account safety, do not use it.**

---

## Your Responsibility

Before using `ValoTracker`, you are responsible for:

1. Reading and understanding Riot Games' current
   [Terms of Service](https://www.riotgames.com/en/terms-of-service)
2. Making your own informed decision about whether using this tool is
   acceptable under those terms
3. Keeping `ValoTracker` up to date — if Riot changes their API or policy, an
   outdated version of `ValoTracker` may behave in ways that were not originally intended

---

## Indemnification

By using this software, you agree to indemnify and hold harmless the author(s)
and contributors of `ValoTracker` from any claim, damage, loss, or expense (including
legal fees) arising from your use of the software or your violation of any
third-party rights or terms of service.

---

## MIT License

This software is released under the MIT License. See [LICENSE](LICENSE) for the
full text. The MIT License further limits the liability of the author(s) to the
maximum extent permitted by law.

---

*This disclaimer was last updated: May 2025.*
