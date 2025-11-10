import { useEffect, useState } from "react";
import { Repo, DocHandle, type AutomergeUrl } from "@automerge/automerge-repo";
import { BrowserWebSocketClientAdapter } from "@automerge/automerge-repo-network-websocket";
import { IndexedDBStorageAdapter } from "@automerge/automerge-repo-storage-indexeddb";

import "./App.css";

interface Doc {
  counter: number;
  notes: string;
  collaborators: string[];
}

function App() {
  const [docHandle, setDocHandle] = useState<DocHandle<Doc> | null>(null);
  const [doc, setDoc] = useState<Doc | null>(null);
  const [username, setUsername] = useState("");
  const [tempUsername, setTempUsername] = useState("");

  useEffect(() => {
    let cleanup: (() => void) | null = null;

    const initRepo = async () => {
      // Initialize repo
      const repo = new Repo({
        network: [new BrowserWebSocketClientAdapter("ws://localhost:3030")],
        storage: new IndexedDBStorageAdapter(),
      });

      // Get or create document
      const hash = window.location.hash.slice(1);
      let handle;

      if (hash) {
        // Load existing document - find returns a Promise
        handle = await repo.find<Doc>(hash as AutomergeUrl);
      } else {
        // Create new document - create returns DocHandle directly
        handle = repo.create<Doc>();
        handle.change((d: Doc) => {
          d.counter = 0;
          d.notes = "";
          d.collaborators = [];
        });
        window.location.hash = handle.url;
      }

      setDocHandle(handle);

      // Wait for handle to be ready
      await handle.whenReady();

      const updateDoc = () => {
        const currentDoc = handle.doc();
        if (currentDoc) {
          setDoc(currentDoc);
        }
      };

      // Initial load
      updateDoc();

      // Subscribe to changes
      const changeListener = () => {
        updateDoc();
      };
      handle.on("change", changeListener);
      cleanup = () => {
        handle.off("change", changeListener);
      };
    };

    initRepo().catch(console.error);

    return () => {
      if (cleanup) cleanup();
    };
  }, []);

  const incrementCounter = () => {
    if (!docHandle) return;
    docHandle.change((d: Doc) => {
      d.counter = (d.counter || 0) + 1;
    });
  };

  const decrementCounter = () => {
    if (!docHandle) return;
    docHandle.change((d: Doc) => {
      d.counter = (d.counter || 0) - 1;
    });
  };

  const updateNotes = (newNotes: string) => {
    if (!docHandle) return;
    docHandle.change((d: Doc) => {
      d.notes = newNotes;
    });
  };

  const joinSession = () => {
    if (!docHandle || !tempUsername) return;
    docHandle.change((d: Doc) => {
      if (!d.collaborators) {
        d.collaborators = [];
      }
      if (!d.collaborators.includes(tempUsername)) {
        d.collaborators.push(tempUsername);
      }
    });
    setUsername(tempUsername);
  };

  const copyUrl = () => {
    navigator.clipboard.writeText(window.location.href);
    alert("URL copied! Share it with others to collaborate.");
  };

  if (!doc) {
    return <div className="loading">Loading document...</div>;
  }

  return (
    <div className="app">
      <header>
        <h1>🚀 Automerge Demo</h1>
        <p className="subtitle">Real-time collaborative document</p>
      </header>

      <div className="container">
        {!username ? (
          <div className="join-section">
            <h2>Join Collaboration</h2>
            <input
              type="text"
              value={tempUsername}
              onChange={(e) => setTempUsername(e.target.value)}
              placeholder="Enter your name"
              onKeyDown={(e) => e.key === "Enter" && joinSession()}
            />
            <button onClick={joinSession} disabled={!tempUsername}>
              Join
            </button>
          </div>
        ) : (
          <>
            <div className="user-info">
              <span>
                👤 Signed in as: <strong>{username}</strong>
              </span>
            </div>

            <div className="section">
              <h2>Collaborative Counter</h2>
              <div className="counter">
                <button onClick={decrementCounter}>-</button>
                <span className="counter-value">{doc.counter || 0}</span>
                <button onClick={incrementCounter}>+</button>
              </div>
            </div>

            <div className="section">
              <h2>Shared Notes</h2>
              <textarea
                value={doc.notes || ""}
                onChange={(e) => updateNotes(e.target.value)}
                placeholder="Start typing... changes sync in real-time!"
                rows={10}
              />
            </div>

            <div className="section">
              <h2>Active Collaborators</h2>
              <div className="collaborators">
                {doc.collaborators && doc.collaborators.length > 0 ? (
                  doc.collaborators.map((name, idx) => (
                    <span key={idx} className="collaborator-badge">
                      {name}
                    </span>
                  ))
                ) : (
                  <p className="empty-state">No collaborators yet</p>
                )}
              </div>
            </div>

            <div className="section">
              <button onClick={copyUrl} className="share-button">
                📋 Copy Share Link
              </button>
            </div>
          </>
        )}
      </div>

      <footer>
        <p>
          Document ID: <code>{docHandle?.url}</code>
        </p>
        <p className="hint">
          Open this page in multiple tabs or share the URL to see real-time
          collaboration!
        </p>
      </footer>
    </div>
  );
}

export default App;
