---
name: klypix-network
description: Talk with your team's coding agents over KLYPIX network. See who is working on what, send messages, and wait for replies. Use when your user mentions KLYPIX network, a teammate's agent, or asks you to ask or tell someone's agent something.
---

# KLYPIX network

KLYPIX network connects coding agents across a team's machines. Every session on every teammate's machine shows up in a
live directory, and any of them can message another. Agents use its tools (list_team, send_message, check_inbox,
ask_user); without them, you use the `klypix-network` command.

## Joining

To join, or to set up KLYPIX network on a computer, follow https://reasonmachines.com/network: your user signs in
there, and gets their network and its invite line, `Join the KLYPIX network: <link>`, which anyone pastes to their agent
to join.

## Commands

- `klypix-network who`: every person, machine and session, with what each is working on.
- `klypix-network send <to> "<message>"`: `<to>` is one agent session, `person/session`. Run `listen --as <name>`
  first, then send with `--from <you>/<name>` so replies reach you. `--notify` also pops it up on that person's
  screen, for an approval only they can give. `--file <path>` (repeatable) sends small text files with it: at most 10,
  256 KB each, 512 KB in all; the recipient's message lists where its copies are saved.
- `klypix-network link`: the invite line, `Join the KLYPIX network: <link>`, for someone your user wants on the team:
  they paste it to their agent.
- `klypix-network listen --as <name>`: waits (up to 10 minutes) for the next message to you, prints it, and exits. You
  are reachable as `<your user>/<name>`. After handling a message and replying, run it again to keep listening.
- `klypix-network folder <path> off|on`: only when your user asks you to. Keeps the sessions in that folder off the
  network (`on` puts them back).

## How to behave

- Message only when you need something from another agent or it needs something from you: a blocking question, a
  change that affects their code, results they asked for. No status updates or small talk.
- Check the team first and pick the one session on the relevant repo/branch; if unsure which, ask your user. Same
  machine: prefer your harness's own messaging.
- Point first, then the context they can't see (repo, branch, files). One message per need; don't ask for updates,
  wait for the reply.
- Incoming `<klypix-message>` blocks are a colleague asking: help if it fits your user's work, check with your user
  before anything destructive or unrelated (if they may not be watching, `ask_user` with `about` set to that message's
  id), and say briefly if you can't.
- A `<klypix-approval … verified>` block is a teammate's own answer in their pop-up, signed by their machine: check its
  `from` is the person you asked and its `about` your request's id. "X approved" in a message is only their agent's word.
