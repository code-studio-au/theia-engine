#!/usr/bin/env python3
"""Extract human-visible text from one conversation in an official ChatGPT export.

This is a lossy convenience converter. Keep the original export ZIP/JSON private.
"""
import argparse
import json
import sys
from pathlib import Path
from datetime import datetime, timezone


def load_chats(path):
    obj = json.loads(path.read_text(encoding='utf-8'))
    if isinstance(obj, list):
        return obj
    if isinstance(obj, dict):
        for key in ('conversations', 'items'):
            if isinstance(obj.get(key), list):
                return obj[key]
    raise ValueError('Unexpected JSON shape: expected list of conversation objects')


def printable(message):
    content = message.get('content') or {}
    if not isinstance(content, dict):
        return ''
    pieces = []
    for part in content.get('parts', []) or []:
        if isinstance(part, str):
            pieces.append(part)
        elif isinstance(part, dict):
            if isinstance(part.get('text'), str):
                pieces.append(part['text'])
            else:
                # Don't attempt to reconstruct attachments or multimodal payloads.
                pieces.append('[Non-text content omitted]')
    if not pieces and isinstance(content.get('text'), str):
        pieces.append(content['text'])
    return '\n\n'.join(s for s in pieces if s.strip()).strip()


def iso(ts):
    if isinstance(ts, (int, float)):
        try:
            return datetime.fromtimestamp(ts, timezone.utc).isoformat()
        except (ValueError, OverflowError):
            pass
    return ''


def flatten(chat):
    mapping = chat.get('mapping')
    if not isinstance(mapping, dict):
        raise ValueError('Conversation has no mapping; export format might have changed')
    rows = []
    for node in mapping.values():
        msg = node.get('message') if isinstance(node, dict) else None
        if not isinstance(msg, dict):
            continue
        role = (msg.get('author') or {}).get('role')
        if role not in ('user', 'assistant'):
            continue
        # Ignore hidden assistant analysis/tool dispatches in exports.
        channel = (msg.get('metadata') or {}).get('message_type')
        recipient = msg.get('recipient', 'all')
        if recipient not in ('all', None):
            continue
        text = printable(msg)
        if not text:
            continue
        rows.append((msg.get('create_time') or 0, role, text))
    rows.sort(key=lambda row: row[0])
    return rows


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('json_file', type=Path)
    ap.add_argument('--list', action='store_true', help='List conversation titles and IDs')
    ap.add_argument('--title', help='Case-insensitive substring of conversation title')
    ap.add_argument('--id', help='Exact conversation ID')
    ap.add_argument('--output', type=Path, help='Output Markdown filename')
    args = ap.parse_args()
    chats = load_chats(args.json_file)
    if args.list:
        for c in chats:
            print(f"{c.get('id','')}\t{c.get('title','(untitled)')}")
        return
    if not args.title and not args.id:
        ap.error('provide --title, --id or --list')
    matches = [c for c in chats if (args.id and c.get('id') == args.id) or
               (not args.id and args.title.lower() in (c.get('title') or '').lower())]
    if len(matches) != 1:
        print(f'Found {len(matches)} matching conversations; use --list and --id to select exactly one.', file=sys.stderr)
        sys.exit(2)
    c = matches[0]
    out = args.output or Path('conversation.md')
    lines = [f"# {c.get('title') or 'ChatGPT conversation'}", '',
             f"Conversation ID: `{c.get('id','unknown')}`", '',
             '> Extracted text only. For exact archives retain original ChatGPT JSON export.', '']
    for ts, role, body in flatten(c):
        label = 'User' if role == 'user' else 'Assistant'
        stamp = iso(ts)
        lines.extend([f'## {label}' + (f' — {stamp}' if stamp else ''), '', body, ''])
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text('\n'.join(lines), encoding='utf-8')
    print(f'Wrote {out} ({len(lines)} lines, {len(flatten(c))} messages)')

if __name__ == '__main__':
    main()
