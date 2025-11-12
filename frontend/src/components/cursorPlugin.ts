import { EditorView, ViewPlugin, ViewUpdate, Decoration, WidgetType } from "@codemirror/view";
import type { DecorationSet } from "@codemirror/view";
import { StateField, StateEffect } from "@codemirror/state";
import { DocHandle } from "@automerge/automerge-repo";

// Peer cursor information
export interface PeerCursor {
  peerId: string;
  name: string;
  color: string;
  position: number;
  selectionStart?: number;
  selectionEnd?: number;
}

// State effect to update peer cursors
export const updatePeerCursors = StateEffect.define<PeerCursor[]>();

// Annotation to mark transactions that come from remote updates
// This helps us avoid resetting decorations during local edits
export const fromRemoteUpdate = StateEffect.define<boolean>();

// Generate a random color for a peer
export function generatePeerColor(peerId: string): string {
  const colors = [
    "#FF6B6B", "#4ECDC4", "#45B7D1", "#FFA07A", "#98D8C8",
    "#F7DC6F", "#BB8FCE", "#85C1E2", "#F8B739", "#52B788"
  ];

  // Use peerId to consistently generate same color
  let hash = 0;
  for (let i = 0; i < peerId.length; i++) {
    hash = peerId.charCodeAt(i) + ((hash << 5) - hash);
  }
  return colors[Math.abs(hash) % colors.length];
}

// Cursor widget class
class CursorWidget extends WidgetType {
  constructor(readonly peer: PeerCursor) {
    super();
  }

  eq(other: CursorWidget) {
    return (
      this.peer.peerId === other.peer.peerId &&
      this.peer.position === other.peer.position &&
      this.peer.name === other.peer.name
    );
  }

  toDOM() {
    const dom = document.createElement("span");
    dom.className = "cm-remote-cursor";
    dom.style.cssText = `
      position: absolute;
      border-left: 2px solid ${this.peer.color};
      height: 1.2em;
      pointer-events: none;
      z-index: 1;
    `;

    // Add cursor label with peer name
    const label = document.createElement("span");
    label.className = "cm-cursor-label";
    label.textContent = this.peer.name;
    label.style.cssText = `
      position: absolute;
      top: -1.5em;
      left: 0;
      background-color: ${this.peer.color};
      color: white;
      padding: 2px 6px;
      border-radius: 4px;
      font-size: 0.75em;
      white-space: nowrap;
      pointer-events: none;
      font-family: sans-serif;
    `;
    dom.appendChild(label);

    return dom;
  }

  ignoreEvent() {
    return true;
  }
}

// Create cursor widget decoration
function cursorWidget(peer: PeerCursor): Decoration {
  return Decoration.widget({
    widget: new CursorWidget(peer),
    side: -1,
  });
}

// Create selection mark decoration
function selectionMark(peer: PeerCursor, from: number, to: number) {
  return Decoration.mark({
    attributes: {
      style: `background-color: ${peer.color}33; border-radius: 2px;`,
    },
  }).range(from, to);
}

// State field to track peer cursors
export const peerCursorField = StateField.define<DecorationSet>({
  create() {
    return Decoration.none;
  },

  update(decorations, tr) {
    // First, map existing decorations through document changes
    // This keeps cursors in the right place during local edits
    decorations = decorations.map(tr.changes);

    // Only rebuild decorations if we received a peer cursor update
    for (const effect of tr.effects) {
      if (effect.is(updatePeerCursors)) {
        const peers = effect.value;
        const newDecorations: any[] = [];

        for (const peer of peers) {
          // Add cursor widget at position
          if (peer.position >= 0 && peer.position <= tr.newDoc.length) {
            newDecorations.push(cursorWidget(peer).range(peer.position));
          }

          // Add selection mark if present
          if (
            peer.selectionStart !== undefined &&
            peer.selectionEnd !== undefined &&
            peer.selectionStart !== peer.selectionEnd
          ) {
            const from = Math.min(peer.selectionStart, peer.selectionEnd);
            const to = Math.max(peer.selectionStart, peer.selectionEnd);

            if (from >= 0 && to <= tr.newDoc.length) {
              newDecorations.push(selectionMark(peer, from, to));
            }
          }
        }

        // Replace decorations with new ones
        decorations = Decoration.set(newDecorations, true);
      }
    }

    return decorations;
  },

  provide: (field) => EditorView.decorations.from(field),
});

// Interface for cursor data stored in Automerge
export interface CursorData {
  peerId: string;
  name: string;
  position: number;
  selectionStart?: number;
  selectionEnd?: number;
  timestamp: number;
}

// Plugin to sync local cursor to Automerge and display remote cursors
export function createCursorSyncPlugin(
  docHandle: DocHandle<any>,
  localPeerId: string,
  localPeerName: string
) {
  return ViewPlugin.fromClass(
    class {
      view: EditorView;
      throttleTimeout: number | null = null;
      updateScheduled: boolean = false;
      peerCursors: Map<string, PeerCursor & { timestamp: number }> = new Map();
      cleanupInterval: number | null = null;

      constructor(view: EditorView) {
        this.view = view;

        // Listen for ephemeral messages (cursor updates from peers)
        docHandle.on("ephemeral-message", this.onEphemeralMessage);

        // Set up periodic cleanup of stale cursors
        this.cleanupInterval = window.setInterval(() => {
          this.cleanupStaleCursors();
        }, 2000); // Check every 2 seconds

        // Send initial cursor position
        this.syncLocalCursor();
      }

      update(update: ViewUpdate) {
        // Only sync cursor on selection changes or when focused
        if (update.selectionSet || update.focusChanged) {
          this.syncLocalCursor();
        }
      }

      syncLocalCursor = () => {
        // Throttle cursor updates to avoid too many messages
        if (this.throttleTimeout) return;

        this.throttleTimeout = window.setTimeout(() => {
          this.throttleTimeout = null;

          const selection = this.view.state.selection.main;

          // Broadcast cursor position via ephemeral message
          const message = {
            type: "cursor",
            peerId: localPeerId,
            name: localPeerName,
            position: selection.head,
            selectionStart: selection.anchor,
            selectionEnd: selection.head,
            timestamp: Date.now(),
          };

          console.log("Broadcasting cursor:", message);
          docHandle.broadcast(message);
        }, 100);
      };

      onEphemeralMessage = (payload: any) => {
        console.log("Received ephemeral message payload:", payload);

        // Ephemeral messages come wrapped with {handle, senderId, message}
        const message = payload.message;
        if (!message) {
          console.log("No message in payload");
          return;
        }

        // Only process cursor messages from other peers
        if (message.type !== "cursor") {
          console.log("Ignoring non-cursor message, type:", message.type);
          return;
        }

        if (message.peerId === localPeerId) {
          console.log("Ignoring own cursor message");
          return;
        }

        console.log("Processing peer cursor from:", message.peerId);

        // Update peer cursor data
        this.peerCursors.set(message.peerId, {
          peerId: message.peerId,
          name: message.name || "Anonymous",
          color: generatePeerColor(message.peerId),
          position: message.position,
          selectionStart: message.selectionStart,
          selectionEnd: message.selectionEnd,
          timestamp: message.timestamp || Date.now(),
        });

        console.log("Peer cursors map now has:", this.peerCursors.size, "cursors");

        // Schedule a decoration update
        this.scheduleUpdate();
      };

      cleanupStaleCursors() {
        const now = Date.now();
        const staleTimeout = 10000; // 10 seconds

        let needsUpdate = false;

        for (const [peerId, cursor] of this.peerCursors.entries()) {
          if (now - cursor.timestamp > staleTimeout) {
            this.peerCursors.delete(peerId);
            needsUpdate = true;
          }
        }

        if (needsUpdate) {
          this.scheduleUpdate();
        }
      }

      scheduleUpdate() {
        if (this.updateScheduled) return;
        this.updateScheduled = true;

        // Use setTimeout to defer the update until after the current event loop
        setTimeout(() => {
          this.updateScheduled = false;
          this.updatePeerCursors();
        }, 0);
      }

      updatePeerCursors() {
        // Convert map to array of peer cursors
        const peers: PeerCursor[] = Array.from(this.peerCursors.values());

        // Update the editor with peer cursors
        try {
          this.view.dispatch({
            effects: updatePeerCursors.of(peers),
          });
        } catch (e) {
          console.error("Error updating peer cursors:", e);
        }
      }

      destroy() {
        if (this.throttleTimeout) {
          clearTimeout(this.throttleTimeout);
        }
        if (this.cleanupInterval) {
          clearInterval(this.cleanupInterval);
        }
        docHandle.off("ephemeral-message", this.onEphemeralMessage);
      }
    }
  );
}
