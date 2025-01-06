import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

interface Area {
  top: number;
  left: number;
  width: number;
  height: number;
}

const config = {
  cursor: {
    forceThreshold: 3000,
    area: {
      left: {
        top: 0.4,
        left: -0.05,
        width: 0.7,
        height: 0.7
      },
      right: {
        top: 0.4,
        left: 0.35,
        width: 0.7,
        height: 0.7
      },
    }
  }
};
const cursorSize = parseInt(getComputedStyle(document.body).getPropertyValue('--cursorsize').split('px')[0], 10);


const keyboardLayout = {
  'type': 'column',
  'elements': [
    {
      'type': 'row',
      'elements': [
        { key: 'escape', label: 'ESC' },
        { key: '1' },
        { key: '2' },
        { key: '3' },
        { key: '4' },
        { key: '5' },
        { key: '6' },
        { key: '7' },
        { key: '8' },
        { key: '9' },
        { key: '0' },
        { key: '-' },
        { key: '=' },
        { key: 'backspace', label: 'Backspace', size: 'u2' },
        { key: 'delete', label: 'Del' }
      ]
    },
    {
      'type': 'row',
      'elements': [
        { key: 'tab', label: 'Tab', size: 'u1_5' },
        { key: 'Q' },
        { key: 'W' },
        { key: 'E' },
        { key: 'R' },
        { key: 'T' },
        { key: 'Y' },
        { key: 'U' },
        { key: 'I' },
        { key: 'O' },
        { key: 'P' },
        { key: '[' },
        { key: ']' },
        { key: '\\', size: 'u1_5' },
        { key: 'home' }
      ]
    },
    {
      'type': 'row',
      'elements': [
        { key: 'capslock', label: 'Caps Lock', size: 'u1_75' },
        { key: 'A' },
        { key: 'S' },
        { key: 'D' },
        { key: 'F' },
        { key: 'G' },
        { key: 'H' },
        { key: 'J' },
        { key: 'K' },
        { key: 'L' },
        { key: ';' },
        { key: '\'' },
        { key: 'return', label: 'Enter', size: 'u2_25' },
        { key: 'page_up', label: 'Pgup' }
      ]
    },
    {
      'type': 'row',
      'elements': [
        { key: 'shift', label: 'Shift', size: 'u2_25' },
        { key: 'Z' },
        { key: 'X' },
        { key: 'C' },
        { key: 'V' },
        { key: 'B' },
        { key: 'N' },
        { key: 'M' },
        { key: ',' },
        { key: '.' },
        { key: '/' },
        { key: 'shift', label: 'Shift', size: 'u1_75' },
        { key: 'up_arrow', label: '⬆️' },
        { key: 'page_down', label: 'Pgdn' }
      ]
    },
    {
      'type': 'row',
      'elements': [
        { key: 'control', label: 'Ctrl', size: 'u1_25' },
        { key: 'super', size: 'u1_25' },
        { key: 'alt', label: 'Alt', size: 'u1_25' },
        { key: 'space', label: '', size: 'u6_25' },
        { key: 'alt', label: 'Alt' },
        { key: null, label: '' },
        { key: null, label: '' },
        { key: 'left_arrow', label: '⬅️' },
        { key: 'down_arrow', label: '⬇️' },
        { key: 'right_arrow', label: '➡️' }
      ]
    },
  ]
};

type KeyState = 'down' | 'up';

async function sendKeys(x: number, y: number, state: KeyState) {
  const elements = document.elementsFromPoint(x, y);
  const keys = elements.filter(element => element.nodeName == 'KEYBOARD-KEY');
  for (const key of keys) {
    if (key instanceof KeyboardKey) {
      await sendKey(key.key, state);
    }
  }
}

async function sendKey(key: string, state: KeyState) {
  return invoke('send_key', {
    key: key,
    state: state
  });
}

async function toggleWindow() {
  return invoke('toggle_window');
}

class KeyboardKey extends HTMLElement {

  button?: HTMLElement;
  key: string;
  label: string;

  constructor(key: string, label: string) {
    super();
    this.key = key;
    this.label = label ?? key;
    this.addEventListener('mousedown', this.onMouseDown.bind(this));
    this.addEventListener('mouseup', this.onMouseUp.bind(this));
  }

  async onMouseDown() {
    await sendKey(this.key, 'down');
  }

  async onMouseUp() {
    await sendKey(this.key, 'up');
  }

  connectedCallback() {
    this.button = document.createElement('button');
    this.button.innerText = this.label;
    this.appendChild(this.button);
  }

  disconnectedCallback() {
    this.innerHTML = '';
  }
}

if ('customElements' in window) {
  window.customElements.define('keyboard-key', KeyboardKey, { extends: 'button' });
}

function isKey(object: any) {
  if ('key' in object) {
    return true;
  }
}

function isLayout(object: any) {
  return ['row', 'column'].includes(object?.type) && Array.isArray(object?.elements);
}

function renderKey(object: any): HTMLElement {
  const result = new KeyboardKey(object?.key, object?.label);
  result.classList.add(object?.key);
  result.classList.add('key');
  if (object?.size) {
    result.classList.add(object.size);
  } else {
    result.classList.add('u1');
  }
  if (object?.id) {
    result.id = object.id;
  }
  return result;
}

function renderRowLayout(object: any): HTMLElement {
  const result = document.createElement('div');
  result.className = 'row';
  for (const element of object?.elements ?? []) {
    const renderedElement = renderKeyboardLayoutElement(element)
    if (renderedElement) {
      result.appendChild(renderedElement);
    }
  }
  return result;
}

function renderColumnLayout(object: any): HTMLElement {
  const result = document.createElement('div');
  result.className = 'column';
  for (const element of object?.elements ?? []) {
    const renderedElement = renderKeyboardLayoutElement(element)
    if (renderedElement) {
      result.appendChild(renderedElement);
    }
  }
  return result;
}

function renderKeyboardLayoutElement(object: any): HTMLElement | undefined {
  if (isKey(object)) {
    return renderKey(object);
  }
  if (isLayout(object)) {
    if (object.type === 'row') {
      return renderRowLayout(object);
    } else if (object.type === 'column') {
      return renderColumnLayout(object);
    }
  }
}

interface SteamDeckDeviceReport {
    lPadX: number;
    lPadY: number;
    lPadForce: number;
    rPadX: number;
    rPadY: number;
    rPadForce: number;
    l4: boolean;
}

/**
 * 
 * @param x s16, from -32k to +32k
 * @param y s16, from -32k to +32k
 * @param area normalized between 0 and 1
 * @returns [x, y] in window coordinates within area
 */
function transform(x: number, y: number, area: Area): [number, number] {
  const width = window.innerWidth;
  const height = window.innerHeight;
  const maxPos = 2 << 14;
  const relX = (1 + x / maxPos) / 2; // [0, 1]
  const relY = (1 - y / maxPos) / 2; // [0, 1]
  return [
    area.left * width + relX * area.width * width, 
    area.top * height + relY * area.height * height, 
  ];
}

/**
 * Moves/hides/shows cursors depending on input.
 * 
 * @param input current input
 * @param lastInput input from last frame/tick
 * @param leftCursor left cursor for left touchpad
 * @param rightCursor right cursor for right touchpad
 */
async function handleTouchpads(
    input: SteamDeckDeviceReport,
    lastInput: SteamDeckDeviceReport,
    leftCursor: HTMLElement,
    rightCursor: HTMLElement) {
  let lPadVisible = input.lPadX != 0 || input.lPadY != 0;
  let rPadVisible = input.rPadX != 0 || input.rPadY != 0;
  let leftCursorX = 0;
  let leftCursorY = 0;
  let rightCursorX = 0;
  let rightCursorY = 0;
  if (lPadVisible) {
    leftCursor.classList.remove('hidden');
    [leftCursorX, leftCursorY] = transform(input.lPadX, input.lPadY, config.cursor.area.left);
    leftCursor.style.top = (leftCursorY - cursorSize / 2) + 'px';
    leftCursor.style.left = (leftCursorX - cursorSize / 2) + 'px';
  } else {
    leftCursor?.classList.add('hidden');
  }
  if (rPadVisible) {
    rightCursor.classList.remove('hidden');
    [rightCursorX, rightCursorY] = transform(input.rPadX, input.rPadY, config.cursor.area.right);
    rightCursor.style.top = (rightCursorY - cursorSize / 2) + 'px';
    rightCursor.style.left = (rightCursorX - cursorSize / 2) + 'px';
  } else {
    rightCursor?.classList.add('hidden');
  }
  if (lastInput.lPadForce < config.cursor.forceThreshold
      && input.lPadForce > config.cursor.forceThreshold) {
    sendKeys(leftCursorX, leftCursorY, 'down');
  } else if (lastInput.lPadForce > config.cursor.forceThreshold
      && input.lPadForce < config.cursor.forceThreshold) {
    sendKeys(leftCursorX, leftCursorY, 'up');
  }
  if (lastInput.rPadForce < config.cursor.forceThreshold
      && input.rPadForce > config.cursor.forceThreshold) {
    sendKeys(rightCursorX, rightCursorY, 'down');
  } else if (lastInput.rPadForce > config.cursor.forceThreshold
      && input.rPadForce < config.cursor.forceThreshold) {
    sendKeys(rightCursorX, rightCursorY, 'up');
  }
}

window.addEventListener('DOMContentLoaded', () => {
  const body = document.querySelector('body');
  const renderedKeyboardLayout = renderKeyboardLayoutElement(keyboardLayout);
  if (renderedKeyboardLayout) {
    body?.appendChild(renderedKeyboardLayout);
  }
  const leftCursor = document.querySelector<HTMLElement>('#leftCursor');
  const rightCursor = document.querySelector<HTMLElement>('#rightCursor');
  if (!leftCursor) {
    throw Error('Left cursor html element missing');
  }
  if (!rightCursor) {
    throw Error('Right cursor html element missing');
  }
  let lastInput: SteamDeckDeviceReport | undefined = undefined;
  // setInterval(() => {
  //   if (lastInput == null) {
  //     return;
  //   }
  //   let [left, top] = transform(lastInput.lPadX, lastInput.lPadY, config.cursor.area.left);
  //   console.log(`left/top transformed: ${left} ${top}`);
  //   console.log(`left/top input: ${lastInput.lPadX} ${lastInput.lPadY}`);
  // }, 5000);
  listen('input', async (event: { payload: SteamDeckDeviceReport }) => {
    let input = event.payload;
    if (!lastInput) {
      lastInput = input;
      return;
    }
    if (input.l4 && !lastInput.l4) {
      toggleWindow();
    }
    handleTouchpads(input, lastInput, leftCursor, rightCursor);
    lastInput = input;
  });
});
