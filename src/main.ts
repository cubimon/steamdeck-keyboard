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
    forceThreshold: 2000,
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
        { key: 'q', label: 'Q' },
        { key: 'w', label: 'W' },
        { key: 'e', label: 'E' },
        { key: 'r', label: 'R' },
        { key: 't', label: 'T' },
        { key: 'y', label: 'Y' },
        { key: 'u', label: 'U' },
        { key: 'i', label: 'I' },
        { key: 'o', label: 'O' },
        { key: 'p', label: 'P' },
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
        { key: 'a', label: 'A' },
        { key: 's', label: 'S' },
        { key: 'd', label: 'D' },
        { key: 'f', label: 'F' },
        { key: 'g', label: 'G' },
        { key: 'h', label: 'H' },
        { key: 'j', label: 'J' },
        { key: 'k', label: 'K' },
        { key: 'l', label: 'L' },
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
        { key: 'z', label: 'Z' },
        { key: 'x', label: 'X' },
        { key: 'c', label: 'C' },
        { key: 'v', label: 'V' },
        { key: 'b', label: 'B' },
        { key: 'n', label: 'N' },
        { key: 'm', label: 'M' },
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

class KeyboardState {

  pressedLeftKeys: KeyboardKey[] = [];
  pressedRightKeys: KeyboardKey[] = [];

  hoveredLeftKeys: KeyboardKey[] = [];
  hoveredRightKeys: KeyboardKey[] = [];
}

async function sendKeys(keys: KeyboardKey[], state: KeyState) {
  keys.forEach(async key => {
    await sendKey(key.key, state);
  });
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

async function triggerHapticPulse() {
  return invoke('trigger_haptic_pulse')
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

function handleHoveredKeys(
    keyboardState: KeyboardState,
    leftKeys: KeyboardKey[],
    rightKeys: KeyboardKey[]) {
  // left
  const releasedLeftKeys = keyboardState.hoveredLeftKeys.filter(
    prevHovKey => leftKeys.indexOf(prevHovKey) < 0);
  const newHoveredLeftKeys = leftKeys.filter(
    newHovKey => keyboardState.hoveredLeftKeys.indexOf(newHovKey) < 0);
  // right
  const releasedRightKeys = keyboardState.hoveredRightKeys.filter(
    prevHovKey => rightKeys.indexOf(prevHovKey) < 0);
  const newHoveredRightKeys = rightKeys.filter(
    newHovKey => keyboardState.hoveredRightKeys.indexOf(newHovKey) < 0);
  [...newHoveredLeftKeys, ...newHoveredRightKeys].forEach((key) => {
    console.log(`adding cursor-hover to ${key.key}`);
    key.classList.add('cursor-hover');
  });
  [...releasedLeftKeys, ...releasedRightKeys].forEach((key) => {
    console.log(`removing cursor-hover to ${key.key}`);
    key.classList.remove('cursor-hover');
  });
  if (newHoveredLeftKeys.length != 0 || newHoveredRightKeys.length != 0) {
    // if new keys are hovered/cursor moved to new key, send haptic feedback
    triggerHapticPulse();
  }
  keyboardState.hoveredLeftKeys = leftKeys;
  keyboardState.hoveredRightKeys = rightKeys;
}

function getKeys(x: number, y: number): KeyboardKey[] {
  const elements = document.elementsFromPoint(x, y);
  const keys = elements.filter(element => element.nodeName == 'KEYBOARD-KEY');
  return keys.filter(key => key instanceof KeyboardKey);
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
 * @param keyboardState information about current state of the keyboard
 * @param input current input
 * @param lastInput input from last frame/tick
 * @param leftCursor left cursor for left touchpad
 * @param rightCursor right cursor for right touchpad
 */
async function handleTouchpads(
    keyboardState: KeyboardState,
    input: SteamDeckDeviceReport,
    lastInput: SteamDeckDeviceReport,
    leftCursor: HTMLElement,
    rightCursor: HTMLElement) {
  let lPadTouched = input.lPadX != 0 || input.lPadY != 0;
  let rPadTouched = input.rPadX != 0 || input.rPadY != 0;
  let leftCursorX = 0;
  let leftCursorY = 0;
  let rightCursorX = 0;
  let rightCursorY = 0;
  if (lPadTouched) {
    leftCursor.classList.remove('hidden');
    [leftCursorX, leftCursorY] = transform(input.lPadX, input.lPadY, config.cursor.area.left);
    leftCursor.style.top = (leftCursorY - cursorSize / 2) + 'px';
    leftCursor.style.left = (leftCursorX - cursorSize / 2) + 'px';
  } else {
    leftCursor?.classList.add('hidden');
  }
  if (rPadTouched) {
    rightCursor.classList.remove('hidden');
    [rightCursorX, rightCursorY] = transform(input.rPadX, input.rPadY, config.cursor.area.right);
    rightCursor.style.top = (rightCursorY - cursorSize / 2) + 'px';
    rightCursor.style.left = (rightCursorX - cursorSize / 2) + 'px';
  } else {
    rightCursor?.classList.add('hidden');
  }
  if (input.lPadForce !== 0) {
    console.log(input.lPadForce);
  }
  if (input.rPadForce !== 0) {
    console.log(input.rPadForce);
  }
  let leftKeys: KeyboardKey[] = [];
  if (lPadTouched) {
    leftKeys = getKeys(leftCursorX, leftCursorY);
  }
  let rightKeys: KeyboardKey[] = [];
  if (rPadTouched) {
    rightKeys = getKeys(rightCursorX, rightCursorY);
  }
  handleHoveredKeys(keyboardState, leftKeys, rightKeys);
  if (lastInput.lPadForce < config.cursor.forceThreshold
      && input.lPadForce > config.cursor.forceThreshold) {
    sendKeys(leftKeys, 'down');
    keyboardState.pressedLeftKeys = leftKeys;
    leftKeys.forEach(key => key.classList.add('pressed'));
  } else if (lastInput.lPadForce > config.cursor.forceThreshold
      && input.lPadForce < config.cursor.forceThreshold) {
    leftKeys.forEach(key => key.classList.remove('pressed'));
    sendKeys(keyboardState.pressedLeftKeys, 'up');
    keyboardState.pressedLeftKeys = [];
  }
  if (lastInput.rPadForce < config.cursor.forceThreshold
      && input.rPadForce > config.cursor.forceThreshold) {
    sendKeys(rightKeys, 'down');
    keyboardState.pressedRightKeys = rightKeys;
    rightKeys.forEach(key => key.classList.add('pressed'));
  } else if (lastInput.rPadForce > config.cursor.forceThreshold
      && input.rPadForce < config.cursor.forceThreshold) {
    rightKeys.forEach(key => key.classList.remove('pressed'));
    sendKeys(keyboardState.pressedRightKeys, 'up');
    keyboardState.pressedRightKeys = [];
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
  let keyboardState = new KeyboardState();
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
    handleTouchpads(
      keyboardState,
      input,
      lastInput,
      leftCursor,
      rightCursor);
    lastInput = input;
  });
});
