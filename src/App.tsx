import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { check } from '@tauri-apps/plugin-updater';
import { ask, message } from '@tauri-apps/plugin-dialog';
import { relaunch } from '@tauri-apps/plugin-process';
import "./App.css";

interface Note { id: number; content: string; }

export default function App() {
  const [notes, setNotes] = useState<Note[]>([]);
  const [newContent, setNewContent] = useState('');

  async function checkForAppUpdates(onUserClick = false) {
    try {
      const update = await check();
      if (update === null) {
        if (onUserClick) {
          await message('Erreur lors de la vérification.', { title: 'Erreur', kind: 'error' });
        }
        return;
      }

      if (update) {
        const ok = await ask(
          `Mise à jour ${update.version} disponible !\n\nNotes : ${update.body}`,
          { title: 'Mise à jour', kind: 'info', okLabel: 'Mettre à jour', cancelLabel: 'Plus tard' }
        );
        if (ok) {
          await update.downloadAndInstall();
          await relaunch();
        }
      } else if (onUserClick) {
        await message('Ton application est à jour.', { title: 'A jour', kind: 'info' });
      }
    } catch (error) {
      console.error('Erreur lors de la vérification des mises à jour :', error);
      if (onUserClick) {
        await message('Erreur lors de la vérification des mises à jour.', { title: 'Erreur', kind: 'error' });
      }
    }
  }

  useEffect(() => {
    // Vérifier les mises à jour au démarrage
    void checkForAppUpdates(false);
    
    // Configuration existante pour les notes
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