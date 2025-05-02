import './index.css';
import { bgm, hit, whistle, start, thrown, title, go } from './sounds.ts';

import { DbConnection, EntityKind, EntityState, ErrorContext, EventContext, Game, GameState, Message, Movable, Vector3D } from './module_bindings';
import { Identity } from '@clockworklabs/spacetimedb-sdk';
import { Timestamp } from '@clockworklabs/spacetimedb-sdk';

const serverUrl = 'ws://localhost:3000';
const moduleName = 'space-vs-time';

type Position = Vector3D;
type EntityEvent = 'Inserted' | 'Updated' | 'Deleted';
let bgmAudio: HTMLAudioElement|undefined;
const willRestartThreshold = -100;

function onConnect(
  conn: DbConnection,
  identity: Identity,
  token: string) {
  localStorage.setItem('auth_token', token);
  console.log('Connected to SpacetimeDB with identity:', identity.toHexString());

  subscribeToQueries(conn, [
    'SELECT * FROM movables',
    'SELECT * FROM games',
    'SELECT * FROM messages'
  ]);
};

function onDisconnect () {
  console.log('Disconnected from SpacetimeDB');
};

function onConnectError (_conn: ErrorContext, err: Error) {
  console.log('Error connecting to SpacetimeDB:', err);
};

function subscribeToQueries(conn: DbConnection, queries: string[]) {
  let count = 0;
  for (const query of queries) {
    conn
      ?.subscriptionBuilder()
      .onApplied(() => {
        count++;
        if (count === queries.length) {
          console.log('SDK client cache initialized.');
          conn.db.movables.onInsert((previous, current) => enqueue(handleMovable.bind(this, 'Inserted', current, previous)));
          conn.db.movables.onUpdate(({}, previous, current) => enqueue(handleMovable.bind(this, 'Updated', current, previous)));
          conn.db.movables.onDelete(({}, previous) => enqueue(handleMovable.bind(this, 'Deleted', previous)));
          conn.db.games.onUpdate(({}, previous, current) => enqueue(handleGame.bind(this, previous, current, false)));
          conn.db.messages.onUpdate(({}, {}, current) => enqueue(handleMessage.bind(this, current, false)))
          for (const movable of conn.db.movables.iter()) {
            enqueue(handleMovable.bind(this, 'Updated', movable));
          }

          for (const game of conn.db.games.iter()) {
            enqueue(handleGame.bind(this, game, game, true));
          }
        }
      })
      .subscribe(query);
  }
}

function handleGame(previous: Game, current: Game, firstRun: boolean = false) {
  updateIntro(current, firstRun ? 0 : 5000);
  if (current.state.tag != previous.state.tag || current.spaceScore != previous.spaceScore || current.timeScore != previous.timeScore) {
    switch(current.state.tag) {
      case GameState.GameStarted.tag:
        playSound(go);
        announce('GO!');
        break;
      case GameState.GameEndedInTie.tag:
        announceGameEnd('IT\'S A TIE!', 'tied', current);
        break;
      case GameState.GameEndedInTimeWin.tag:
        announceGameEnd('TIME WINS!', 'timewins', current);
        break;
      case GameState.GameEndedInSpaceWin.tag:
        announceGameEnd('SPACE WINS!', 'spacewins', current);
        break;
      case GameState.SpaceScored.tag:
      case GameState.TimeScored.tag:
        playSound(hit);
        break;
    }
  }

  updateAllDigits(current);
}

function updateIntro(game: Game, timeout: number) {
  if (game.remaining >= willRestartThreshold) {
    hideIntro();
  } else {
    showIntro(timeout);
  }
}

function hideIntro() {
  const intro = document.getElementById('intro');
  if (intro && (intro.hidden || !intro.classList.contains('hidden'))) {
    if (!intro.hidden) {
      playSound(start);
      intro.classList.add('hidden');
    }

    if (!bgmAudio) {
      playBgm();
    }
  }
}

function showIntro(timeout: number) {
  const intro = document.getElementById('intro');
  if (intro && intro.classList.contains('hidden')) {
    delay(() => {
      intro.hidden = false;
      stopBgm();
      delay(playSound.bind(this, title, false), 1000);
      intro.classList.remove('hidden');
    }, timeout);
  }
}

function delay(action: () => void, delay: number) {
  if (delay <= 0) {
    enqueue(action);
  } else {
    setTimeout(enqueue.bind(this, action), delay);
  }
}

function updateAllDigits(game: Game) {
  if (game.remaining > 0) {
    updateDigits('space-score', game.spaceScore);
    updateDigits('time-score', game.timeScore);
    updateDigits('clock', game.remaining)
  } else if (game.remaining >= -3 && game.remaining < 0) {
    updateDigits('space-score', null);
    updateDigits('time-score', null);
    announce(Math.abs(game.remaining).toString(), 'countdown');
  } else {
    updateDigits('clock', null);
  }
}

function announceScoreUpdate({spaceScore, timeScore}: Game) {
  announce(`${spaceScore} - ${timeScore}`);
}

function updateDigits(id: 'space-score'|'time-score'|'clock', value: number|null) {
  const scoreElement = document.getElementById(id);
  if (scoreElement) {
    let tenths = '';
    let ones = '';

    if (value !== null) {
      if (value >= 100) {
        tenths = '-';
        ones = '-';
      } else {
        tenths = Math.floor(value / 10).toString();
        ones = Math.floor(value % 10).toString();
      }
    }

    scoreElement.dataset.tenths = tenths;
    scoreElement.dataset.ones = ones;
  }
}

function handleMessage(message: Message, isMvp: boolean) {
  const element = document.getElementById(`${message.name}-bubble`);
  if (!element) {
    return;
  }

  element.innerText = message.text;
  if (isMvp) {
    element.classList.add('mvp');
  } else {
    element.classList.remove('mvp');
  }

  delay(() => { element.innerText = ''; }, isMvp ? 5000 : 2000);
}

function announce(text: String, style: 'countdown'|'spacewins'|'timewins'|'tied'|undefined = undefined) {
  let announcement = document.getElementById('announcement');
  if (!announcement) {
    return;
  }
  const parent = announcement.parentElement;
  if (!parent) {
    return;
  }
  announcement.remove();
  announcement = parent.ownerDocument.createElement('div');
  announcement.id = 'announcement';
  if (style) {
    announcement.classList.add(style);
  }
  for (let i = 0; i < text.length; i++) {
    const letter = text.substring(i, i + 1).replace(' ', ' ');
    const element = announcement.ownerDocument.createElement('span');
    element.innerText = letter;
    element.dataset.text = letter;
    announcement.appendChild(element);
  }
  parent.appendChild(announcement);
}

function announceGameEnd(title: string, style: 'spacewins' | 'timewins' | 'tied', current: Game) {
  playSound(whistle);
  announceScoreUpdate(current);
  setTimeout(() => announce(title, style), 1600);
  if (current.mvp) {
    updateMvp(current.mvp);
  }
}

function handleMovable(event: EntityEvent, current: Movable, previous: Movable|undefined) {
  switch (event) {
    case 'Inserted':
    case 'Updated':
      const element = ensureMovableExists(current.name, current.kind);
      updateMovable(element, current.position, current.state, previous?.state);
      break;
    case 'Deleted':
      document.getElementById(current.name)?.remove();
      break;
    default:
      throw new Error(`Event type ${event} not supported`);
  }
}

function enqueue(action: () => void) {
  window.requestAnimationFrame(action);
}

function ensureMovableExists (name: string, kind: EntityKind): HTMLElement {
  let movable = document.getElementById(name);
  if (!movable) {
    const field = document.getElementById('field');
    if (!field) throw new Error('Could not find field');
    movable = document.createElement('div');
    movable.id = name;
    movable.classList.add(...getClassNamesForEntityKind(kind));
    
    if (movable.classList.contains('player')) {
      const bubble = document.createElement('div');
      bubble.id = `${name}-bubble`;
      bubble.classList.add('bubble');
      movable.appendChild(bubble);
    }

    field.appendChild(movable);
  }
  return movable;
}
function getClassNamesForEntityKind(entityKind: EntityKind) {
  switch (entityKind.tag) {
    case EntityKind.Ball.tag:
      return ['movable', 'ball'];
    case EntityKind.SpacePlayer.tag:
      return ['movable', 'space', 'player'];
    case EntityKind.TimePlayer.tag:
      return ['movable', 'time', 'player'];
    default:
      throw new Error(`EntityKind ${entityKind.tag} not supported`);
  }
}

function updateMovable(movable: HTMLElement, position: Position, currentState: EntityState, previousState: EntityState|undefined) {
  movable.dataset.x = position.x.toString();
  movable.dataset.y = position.y.toString();
  movable.dataset.z = position.z.toString();
  movable.style.zIndex = (99999 - Math.round(position.y * 10000)).toString();
  movable.dataset.state = currentState.tag;
  movable.style.setProperty("--zoom", (0.5 + (1 - (position.z / 20)) * 0.5).toString());

  if (currentState.tag != previousState?.tag && currentState.tag == EntityState.Throwing.tag) {
    setTimeout(() => playSound(thrown), 200);
  }
}

function updateMvp(name: string) {
  handleMessage({ name, text: "MVP", timestamp: Timestamp.UNIX_EPOCH }, true);
}

function playSound(sound, loop: boolean = false): HTMLAudioElement {
  const audio = new Audio(sound);
  audio.loop = loop;
  audio.play();
  return audio;
}

function playBgm() {
  stopBgm();
  bgmAudio = playSound(bgm, true);
}

function stopBgm() {
  if (bgmAudio) {
    bgmAudio.pause();
    bgmAudio = undefined;
  }
}

DbConnection.builder()
.withUri(serverUrl)
.withModuleName(moduleName)
.withToken(localStorage.getItem('auth_token') || '')
.onConnect(onConnect)
.onDisconnect(onDisconnect)
.onConnectError(onConnectError)
.build();
