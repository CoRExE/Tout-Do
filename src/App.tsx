import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import "./App.css";

interface Note { id: number; content: string; }

export default function App() {
  const [notes, setNotes] = useState<Note[]>([]);
  const [newContent, setNewContent] = useState('');

  useEffect(() => {
    void (async () => {
      const appWindow = await getCurrentWindow();
      const existing = await invoke<Note[]>('list_notes');
      setNotes(existing);

      const unlisten = await appWindow.listen<Note[]>('notes_updated', event => {
        setNotes(event.payload);
      });
      return () => {
        unlisten();
      };
    })();
  }, []);

  const add = async () => {
    if (!newContent.trim()) return;
    await invoke('add_note', { content: newContent });
    setNewContent('');
  };

  const del = async (id: number) => {
    await invoke('delete_note', { id });
  };

  return (
    <div>
      <h1>Notes</h1>
      <input value={newContent} onChange={e => setNewContent(e.target.value)} />
      <button style={{ margin: '10px' }} onClick={add}>Ajouter</button>
      <ul>
        {notes.map(n => (
          <li key={n.id}>
            {n.content}
            <button style={{ margin: '10px' }} onClick={() => del(n.id)}>🗑️</button>
          </li>
        ))}
      </ul>
    </div>
  );
}