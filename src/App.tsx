import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { check } from '@tauri-apps/plugin-updater';
import { ask, message } from '@tauri-apps/plugin-dialog';
import { relaunch } from '@tauri-apps/plugin-process';
import { DragDropContext, Droppable, Draggable, DropResult } from "@hello-pangea/dnd";
import "./App.css";

interface Note {
  id: number;
  content: string;
  /** Whether the note is pinned to the top of the list */
  pinned?: boolean;
}

export default function App() {
  const [notes, setNotes] = useState<Note[]>([]);
  /** Order notes so that pinned notes appear first */
  const orderNotes = (list: Note[]) =>
    [...list].sort((a, b) =>
      a.pinned === b.pinned ? 0 : a.pinned ? -1 : 1
    );
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
      setNotes(orderNotes(existing));

      const unlisten = await appWindow.listen<Note[]>('notes_updated', event => {
        setNotes(orderNotes(event.payload));
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

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      void add();
    }
  };

  const del = async (id: number) => {
    await invoke('delete_note', { id });
  };

  const togglePin = async (id: number) => {
    await invoke('toggle_pin', { id });
  };

  const onDragEnd = async (result: DropResult) => {
    if (!result.destination) return;
    
    const items = Array.from(notes);
    const [reorderedItem] = items.splice(result.source.index, 1);
    items.splice(result.destination.index, 0, reorderedItem);
    
    // Update the local state immediately for a responsive UI
    setNotes(items);
    
    try {
      // Send the new order to the backend
      await invoke('reorder_notes', { 
        orderedIds: items.map(note => note.id) 
      });
    } catch (error) {
      console.error('Failed to reorder notes:', error);
      // If there's an error, refresh the notes from the backend
      const updatedNotes = await invoke<Note[]>('list_notes');
      setNotes(orderNotes(updatedNotes));
    }
  };

  return (
    <div>
      <h1>Notes</h1>
      <input 
        value={newContent} 
        onChange={e => setNewContent(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Type a note and press Enter"
      />
      <button style={{ margin: '10px' }} onClick={add}>Ajouter</button>
      <DragDropContext onDragEnd={onDragEnd}>
        <Droppable droppableId="notes">
          {(provided) => (
            <ul {...provided.droppableProps} ref={provided.innerRef}>
              {notes.map((n, idx) => (
                <Draggable key={n.id} draggableId={n.id.toString()} index={idx}>
                  {(provided) => (
                    <li
                      ref={provided.innerRef}
                      {...provided.draggableProps}
                      {...provided.dragHandleProps}
                      style={{
                        ...provided.draggableProps.style,
                        listStyle: 'none',
                        padding: '10px',
                        margin: '5px 0',
                        backgroundColor: '#f5f5f5',
                        borderRadius: '4px',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between'
                      }}
                    >
                      <span>{n.content}</span>
                      <div>
                        <button
                          style={{ margin: '0 5px' }}
                          onClick={(e) => {
                            e.stopPropagation();
                            void togglePin(n.id);
                          }}
                          title={n.pinned ? 'Désépingler' : 'Épingler'}
                        >
                          {n.pinned ? '📌' : '📍'}
                        </button>
                        <button 
                          style={{ margin: '0 5px' }} 
                          onClick={(e) => {
                            e.stopPropagation();
                            void del(n.id);
                          }}
                        >
                          🗑️
                        </button>
                      </div>
                    </li>
                  )}
                </Draggable>
              ))}
              {provided.placeholder}
            </ul>
          )}
        </Droppable>
      </DragDropContext>
    </div>
  );
}