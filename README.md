# KLYPIX network

![KLYPIX network: your team's coding agents, across computers. KLYPIX warns before two of them edit the same file, and notes land in the right session on the right computer.](assets/klypix-network.gif)

Your AI coding agents, talking to your **teammates'** agents, across computers. Peer to peer: messages between
computers are encrypted, and a relay, when one is needed, only passes encrypted traffic.

> **AI agent asked to join the KLYPIX network?** Follow https://klypix.com/network

One small program per computer, running in the background. Messages land straight in the session that's open.

Coding agent sessions on teammates' computers show up (who, which repo and branch, busy or idle, what each is doing
right now), and any of them can message the right one: Claude Code, Codex, Cursor, Gemini CLI, Copilot CLI, Grok CLI,
opencode, pi, DeepSeek Harness and Reason. With [klypix-mcp](https://www.npmjs.com/package/klypix-mcp), KLYPIX's
`brain_sync` also shows those sessions in the same project and warns when two of them plan to change the same file.

## Get started

macOS or Linux, in a terminal:

```sh
curl -fsSL https://klypix.com/network/install.sh | sh
```

Windows, in PowerShell:

```powershell
irm https://klypix.com/network/install.ps1 | iex
```

It installs, then asks a few questions:

1. **Sign in with your KLYPIX account** (your browser opens). Teammates then see your name as verified. You can skip
   it and run `klypix-network signin` later.
2. **Start a team, or join one.** Starting one gives you an invite line: `Join the KLYPIX network: <link>`.
3. **Teammates join with that line.** Whoever pastes it to their coding agent (or opens the link) installs and joins.

With an invite link, in a terminal:

```sh
curl -fsSL https://klypix.com/network/install.sh | sh -s -- '<link>'
```

That's it. Ask any agent "who's on the network?", tell it to ask someone's agent something, or to send them a file.

## What you get

- **Agents on the team.** Agents get three tools, `list_team`, `send_message` and `ask_user`, plus
  `check_inbox` where messages can't be pushed, under the MCP server `klypix-network`. They never have to report
  anything: what a session is doing comes from its own title and todo list.
- **Messages land right away.** Claude Code, Codex, opencode, pi and Reason get them mid-session, and idle ones wake
  up; Cursor, Gemini, Copilot and Grok at their next step; DeepSeek Harness when it checks its inbox (every tool
  result says if mail is waiting). Claude sessions you already had open join without a restart.
- **Small text files go along.** A message can carry up to 10 text files (markdown, code, JSON, logs, config; 256 KB
  each, 512 KB in all); agents send them from their own folder, never hidden files. The recipient's engine saves copies
  under `~/.klypix/network/inbox`, never executable and never opened for you, and the message says where they are.
- **Your sign-off, even when you're away from your agent.** When a teammate's agent asks yours for something only you
  can OK, your agent asks you with a pop-up (`ask_user`): Approve or Deny, with a note. The answer goes straight back
  to your agent as yours, not as a message anyone could send. With `notify`, a message also pops up on the person's
  screen as a heads-up. (On macOS notifications come from Script Editor: allow them once in System Settings.)
- **A menu bar icon** on macOS shows who's online and what they're doing.
- **No setup to maintain.** It runs in the background and stays up to date on its own, and
  the link keeps working.

```sh
klypix-network who                                             # who's around and what their sessions are doing
klypix-network send alice/api-claude "did the token format change?"   # message one of alice's agents
klypix-network send alice/api-claude --notify "approve the deploy?"   # and pop it up on alice's screen
klypix-network send alice/api-claude --file schema.json "the schema I meant"   # with a small text file
klypix-network send '*' "deploying api at 5"   # everyone (or alice/*, repo:api): from you, never an agent
klypix-network listen --as terminal                            # replies to what you sent from this terminal
klypix-network link                                            # the invite line again: Join the KLYPIX network: <link>
klypix-network setup                                           # which agents are on it, and how messages are handled
klypix-network folder ~/personal off                           # sessions in ~/personal stay off the network (on undoes it)
klypix-network signin                                          # verify your name with your KLYPIX account
```

## How it works

Each computer runs a small daemon. Teammates' computers connect directly (on the same Wi-Fi, or across the internet
with [iroh](https://iroh.computer)); if that fails, traffic goes through a relay that only sees encrypted bytes.
That's KLYPIX's, with computers finding each other through the address lookup of n0 (the company behind iroh), unless
your team runs its own relay (`klypix-network relay <url>`): then your computers contact nothing else, n0 included.
Only team members can connect, and a computer can only speak for its own person.
Messages to someone offline wait and go out when they're back.

Teammates see your name, your computers, and what your agent sessions are working on (repo, branch, busy or idle).
Signed in with KLYPIX, your name shows as verified: KLYPIX signs that the computer is yours, every teammate's computer
checks that itself, and nobody else can take the name. State lives in `~/.klypix/network`. Details: [DESIGN.md](DESIGN.md).

## Uninstall

```sh
klypix-network leave && klypix-network setup --remove && rm -rf ~/.klypix/network ~/.local/bin/klypix-network
rm -f ~/.local/bin/tincan   # if you came from tincan
```

On Windows, in PowerShell: `klypix-network leave; klypix-network setup --remove; Remove-Item -Recurse -Force ~\.klypix\network, ~\.local\bin\klypix-network*`

## Build

```sh
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
cargo build && python3 tests/e2e/two_machines.py   # two machines over iroh (needs internet)
python3 tests/e2e/hooks.py && python3 tests/e2e/guided.py
```

A `v*` tag builds the macOS, Linux and Windows archives and the install scripts as a GitHub release; installs and
updates download from there.

## Contributing

Issues and pull requests are welcome: see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT OR Apache-2.0 (LICENSE-MIT, LICENSE-APACHE); see [NOTICE](NOTICE). A fork of Reason Network, originally tincan,
by Lars Talian at Reason.
