import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

async function readConfig() {
  return invoke('read_config');
}

async function sendKey(
    key: string,
    state: KeyState) {
  return invoke('send_key', {
    key: key,
    state: state,
  });
}

async function triggerHapticPulse(pad: number) {
  return invoke('trigger_haptic_pulse', {
    pad: pad
  });
}

async function log(level: string, message: string) {
  return invoke('log', {
    level: level,
    message: message,
  });
}

interface Area {
  top: number;
  left: number;
  width: number;
  height: number;
}

interface KeyLayout {
  key: string | null;
  label?: string;
  type?: null | 'osm';
  size?: string;
}

interface KeyboardLayout {
  type: 'column' | 'row';
  elements: KeyboardLayout[] | KeyLayout[];
}

/**
 * Topmost layout which contains an additional flag if active.
 */
interface KeyboardLayer extends KeyboardLayout {
  active: boolean;
}

/**
 * layer name => rendered element and keyboard layout.
 */
interface RenderedKeyboardLayout {
  [key: string]: {
    element: HTMLElement,
    keyboardLayer: KeyboardLayer
  }
}

const defaultKeyboardLayout: KeyboardLayout = {
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
        { key: 'shift', label: 'Shift', size: 'u2_25', type: 'osm' },
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
        { key: 'shift', label: 'Shift', size: 'u1_75', type: 'osm' },
        { key: 'up_arrow', label: '⬆️' },
        { key: 'page_down', label: 'Pgdn' }
      ]
    },
    {
      'type': 'row',
      'elements': [
        { key: 'control', label: 'Ctrl', size: 'u1_25', type: 'osm' },
        { key: 'meta', size: 'u1_25', type: 'osm' },
        { key: 'alt', label: 'Alt', size: 'u1_25', type: 'osm' },
        { key: 'space', label: '', size: 'u6_25' },
        { key: 'alt', label: 'Alt', type: 'osm' },
        { key: null, label: '' },
        { key: null, label: '' },
        { key: 'left_arrow', label: '⬅️' },
        { key: 'down_arrow', label: '⬇️' },
        { key: 'right_arrow', label: '➡️' }
      ]
    },
  ]
};

interface CursorConfig {
  forceThreshold: number;
  hapticOnHover: boolean;
  hapticOnClick: boolean;
  area: {
    left: {
      top: number;
      left: number;
      width: number;
      height: number;
    },
    right: {
      top: number;
      left: number;
      width: number;
      height: number;
    }
  }
};

interface Config {
  deadzone: number;
  cursor: CursorConfig;
  layers: {
    [key: string]: KeyboardLayout;
  };
}

const defaultConfig: Config = {
  deadzone: 500,
  cursor: {
    forceThreshold: 2000,
    hapticOnHover: true,
    hapticOnClick: true,
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
    },
  },
  layers: {
    default: defaultKeyboardLayout,
  }
};
const cursorSize = parseInt(getComputedStyle(document.body).getPropertyValue('--cursorsize').split('px')[0], 10);

type KeyState = 'down' | 'up';

class KeyboardStateChange {
  key: KeyboardKey;
  state: KeyState;
  time: Date;

  constructor(
      key: KeyboardKey,
      state: KeyState,
      now: Date) {
    this.key = key;
    this.state = state;
    this.time = now;
  }
}

type KeyStateChangeListener = (
  key: KeyboardKey,
  state: KeyState,
  now: Date) => void;

class KeyboardState {

  pressedLeftKeys: KeyboardKey[] = [];
  pressedRightKeys: KeyboardKey[] = [];

  hoveredLeftKeys: KeyboardKey[] = [];
  hoveredRightKeys: KeyboardKey[] = [];

  keyboardStateChanges: KeyboardStateChange[] = [];

  beforeKeyStateChangeListener: KeyStateChangeListener[] = [];
  afterKeyStateChangeListener: KeyStateChangeListener[] = [];

  heldOsmKeys: Set<KeyboardKeyOsm> = new Set();
  keyboardLayers: { [key: string]: KeyboardLayer } = {};

  isOsmShifted(): boolean {
    for (let heldOsmKey of this.heldOsmKeys) {
      if (heldOsmKey.isShift()) {
        return true;
      }
    }
    return false;
  }

  isShifted(): boolean {
    if (this.isOsmShifted()) {
      return true;
    }
    for (let pressedLeftKey of this.pressedLeftKeys) {
      if (pressedLeftKey.isShift()) {
        return true;
      }
    }
    for (let pressedRightKey of this.pressedRightKeys) {
      if (pressedRightKey.isShift()) {
        return true;
      }
    }
    return false;
  }

  async keyStateChanges(
      keys: KeyboardKey[],
      state: KeyState,
      now: Date) {
    keys.forEach(async key => {
      await this.keyStateChange(key, state, now);
    });
  }

  async keyStateChange(
      key: KeyboardKey,
      state: KeyState,
      now: Date) {
    this.publishBeforeKeyStateChange(key, state, now);
    this.updateHistory(key, state, now);
    const result = await key.keyStateChange(state, now);
    this.publishAfterKeyStateChange(key, state, now);
    return result;
  }

  enableLayer(layerName: string) {
    if (!(layerName in this.keyboardLayers)) {
      log('error', `Layer name ${layerName} not in layer states`);
      return;
    }
    log('debug', `Enabling layer ${layerName}`);
    this.keyboardLayers[layerName].active = true;
    enableLayer(layerName);
    this.updateLayerStackTransparency();
  }

  disableLayer(layerName: string) {
    if (!(layerName in this.keyboardLayers)) {
      log('error', `Layer name ${layerName} not in layer states`);
      return;
    }
    log('debug', `Disabling layer ${layerName}`);
    this.keyboardLayers[layerName].active = false;
    disableLayer(layerName);
    this.updateLayerStackTransparency();
  }

  /**
   * Updates layer transparency depending on active flag,
   * so that only the topmost layer is visible, but others below are still clickable.
   */
  updateLayerStackTransparency() {
    log('trace', 'Updating layer stack transparency');
    const layers = Object.entries(this.keyboardLayers)
      .filter(([_layerName, keyboardLayer]) => keyboardLayer.active);
    log('trace', `layers size ${layers.length}`);
    unhideLayer(layers[0][0]);
    layers
      .slice(1)
      .forEach(([layerName, _keyboardLayer]) => {
        hideLayer(layerName);
      });
  }

  publishBeforeKeyStateChange(
      key: KeyboardKey,
      state: KeyState,
      now: Date) {
    this.beforeKeyStateChangeListener.forEach(listener => {
      listener(key, state, now);
    })
  }

  publishAfterKeyStateChange(
      key: KeyboardKey,
      state: KeyState,
      now: Date) {
    this.afterKeyStateChangeListener.forEach(listener => {
      listener(key, state, now);
    })
  }

  subscribeBeforeKeyStateChange(
      listener: KeyStateChangeListener) {
    this.beforeKeyStateChangeListener.push(listener);
  }

  subscribeAfterKeyStateChange(
      listener: KeyStateChangeListener) {
    this.afterKeyStateChangeListener.push(listener);
  }

  holdOsmKey(key: KeyboardKeyOsm) {
    this.heldOsmKeys.add(key);
  }

  unholdOsmKey(key: KeyboardKeyOsm) {
    this.heldOsmKeys.delete(key);
  }

  updateHistory(
      key: KeyboardKey,
      state: KeyState,
      now: Date) {
    this.keyboardStateChanges.push(
      new KeyboardStateChange(key, state, now));
    this.keyboardStateChanges = this.keyboardStateChanges.filter(
      keyStateChange => now.getTime() - keyStateChange.time.getTime() < 2000);
  }
}

/**
 * Options for all keys.
 */
interface KeyboardKeyOptionsGeneric {
  label?: string;
  /**
   * e.g. u1, u1.5 or u1.75
   */
  size?: string;
  id?: string;
}

/**
 * Press key.
 */
interface KeyboardKeyOptionKey {
  key: string;
}

/**
 * Change to layer.
 */
interface KeyboardKeyOptionLayer {
  layer: string;
}

/**
 * Transparent key/neither key nor layer change.
 * Show key behind or nothing
 */
interface KeyboardKeyOptionTrans {
}

type KeyboardKeyOptions =
  KeyboardKeyOptionsGeneric & (
  KeyboardKeyOptionKey |
  KeyboardKeyOptionLayer |
  KeyboardKeyOptionTrans
);

class KeyboardKey extends HTMLElement {

  button?: HTMLElement;
  keyboardState: KeyboardState;
  keyboardLayer: KeyboardLayer;
  key?: string;
  layer?: string;
  label: string;

  constructor(
      keyboardState: KeyboardState,
      keyboardLayer: KeyboardLayer,
      options: KeyboardKeyOptions) {
    super();
    this.keyboardState = keyboardState;
    this.keyboardLayer = keyboardLayer;
    if ('key' in options) {
      this.key = options.key;
    }
    if ('layer' in options) {
      this.layer = options.layer;
    }
    this.label = options?.label ?? this.key ?? "";
    this.addEventListener('mousedown', this.onMouseDown.bind(this));
    this.addEventListener('mouseup', this.onMouseUp.bind(this));
    if ('key' in options) {
      this.classList.add(options?.key);
    }
    this.classList.add('key');
    if (options?.size) {
      this.classList.add(options.size);
    } else {
      this.classList.add('u1');
    }
    if (options?.id) {
      this.id = options.id;
    }
    if (!this.key && !this.layer) {
      this.classList.add('transparent');
    }
  }

  async onMouseDown(event: MouseEvent) {
    log('trace', `Mouse down event { x: ${event.x}, y: ${event.y} }`);
    getKeys(event.x, event.y).forEach(key => {
      this.keyboardState.keyStateChange(key, 'down', new Date());
    });
  }

  async onMouseUp(event: MouseEvent) {
    log('trace', `Mouse up event { x: ${event.x}, y: ${event.y} }`);
    getKeys(event.x, event.y).forEach(key => {
      this.keyboardState.keyStateChange(key, 'up', new Date());
    });
  }

  connectedCallback() {
    this.button = document.createElement('button');
    this.button.innerText = this.label;
    this.appendChild(this.button);
  }

  disconnectedCallback() {
    this.innerHTML = '';
  }

  isShift(): boolean {
    return this.key === 'shift';
  }

  /**
   * if this has no key or layer.
   *
   * @returns true if key is transparent
   */
  isTrans(): boolean {
    return !this.key && !this.layer;
  }

  /**
   * if layer is active.
   *
   * @returns true if layer is active
   */
  isActive(): boolean {
    return this.keyboardLayer.active;
  }

  async keyStateChange(
      state: KeyState,
      _now: Date) {
    if (state === 'down') {
      this.classList.add('pressed');
    } else if (state === 'up') {
      this.classList.remove('pressed');
    }
    if (this.key) {
      let keyChar = this.key
      if (this.keyboardState.isShifted() && keyChar.length == 1) {
        // to upper case single characters if shifted
        // e.g. 'l' to 'L' or 'i' to 'I'
        keyChar = keyChar.toUpperCase();
      }
      return sendKey(keyChar, state);
    }
    if (this.layer) {
      if (state === 'down') {
        this.keyboardState.enableLayer(this.layer);
      } else if (state === 'up') {
        this.keyboardState.disableLayer(this.layer);
      }
    }
  }

  toString(): string {
    return `KeyboardKey(
      key: ${this.key},
      layer: ${this.layer},
      label: ${this.label}
    )`;
  }
}

enum OsmState {
  RELEASED,
  SINGLE_HOLD,
  DOUBLE_HOLD,
};

class KeyboardKeyOsm extends KeyboardKey {

  state: OsmState;
  stateStartTime: Date;

  constructor(
      keyboardState: KeyboardState,
      keyboardLayer: KeyboardLayer,
      options: KeyboardKeyOptions) {
    super(keyboardState, keyboardLayer, options);
    this.state = OsmState.RELEASED;
    this.stateStartTime = new Date();
    this.keyboardState.subscribeAfterKeyStateChange(
      this.afterKeyStateChange.bind(this));
  }

  afterKeyStateChange(
      key: KeyboardKey,
      state: KeyState,
      _now: Date) {
    if (key instanceof KeyboardKeyOsm) {
      return;
    }
    if (this.state == OsmState.SINGLE_HOLD && state == 'up') {
      if (this.key) {
        sendKey(this.key, 'up');
      } else if (this.layer) {
        this.keyboardState.disableLayer(this.layer);
      }
      this.state = OsmState.RELEASED;
      this.keyboardState.unholdOsmKey(this);
    }
  }

  async keyStateChange(
      state: KeyState,
      now: Date) {
    const isDoubleTap = this.keyboardState.keyboardStateChanges
      .filter(sc => sc.key instanceof KeyboardKeyOsm)
      .filter(sc => sc.state == 'down')
      .filter(sc => sc.time.getTime() > now.getTime() - 1000)
      .filter(sc => sc.time.getTime() > this.stateStartTime.getTime())
      .length >= 2;
    if (isDoubleTap && state === 'up') {
      this.state = OsmState.DOUBLE_HOLD;
      this.stateStartTime = now;
      log('debug', 'double hold now');
      this.keyboardState.holdOsmKey(this);
      return;
    }
    if (this.state == OsmState.DOUBLE_HOLD && state === 'up') {
      log('debug', 'unhold double hold');
      let result = super.keyStateChange(state, now);
      this.state = OsmState.RELEASED;
      this.keyboardState.unholdOsmKey(this);
      return result;
    }
    if ([OsmState.SINGLE_HOLD, OsmState.DOUBLE_HOLD].includes(this.state)) {
      return;
    }
    log('trace', 'single hold now');
    let result = super.keyStateChange(state, now);
    this.state = OsmState.SINGLE_HOLD;
    this.keyboardState.holdOsmKey(this);
    return result;
  }

  toString(): string {
    return `KeyboardKeyOsm(
      key: ${this.key},
      layer: ${this.layer},
      label: ${this.label},
      state: ${this.state}
    )`;
  }
}

if ('customElements' in window) {
  window.customElements.define('keyboard-key', KeyboardKey, { extends: 'button' });
  window.customElements.define('keyboard-key-osm', KeyboardKeyOsm, { extends: 'button' });
}

function isKey(object: any) {
  if ('key' in object || 'layer' in object) {
    return true;
  }
  return !('elements' in object);
}

function isLayout(object: any) {
  return ['row', 'column'].includes(object?.type) && Array.isArray(object?.elements);
}

function renderKey(
    keyboardState: KeyboardState,
    keyboardLayer: KeyboardLayer,
    object: any): HTMLElement {
  let result = null;
  switch (object?.type) {
    case 'osm':
    case 'layer':
      result = new KeyboardKeyOsm(
        keyboardState,
        keyboardLayer,
        object);
      break;
    default:
      result = new KeyboardKey(
        keyboardState,
        keyboardLayer,
        object);
  }
 return result;
}

function renderRowLayout(
    keyboardState: KeyboardState,
    keyboardLayer: KeyboardLayer,
    object: any): HTMLElement {
  const result = document.createElement('div');
  result.classList.add('row');
  result.classList.add('layout');
  for (const element of object?.elements ?? []) {
    const renderedElement = renderKeyboardLayoutElement(
      keyboardState, keyboardLayer, element)
    if (renderedElement) {
      result.appendChild(renderedElement);
    }
  }
  return result;
}

function renderColumnLayout(
    keyboardState: KeyboardState,
    keyboardLayer: KeyboardLayer,
    object: any): HTMLElement {
  const result = document.createElement('div');
  result.classList.add('column');
  result.classList.add('layout');
  for (const element of object?.elements ?? []) {
    const renderedElement = renderKeyboardLayoutElement(
      keyboardState, keyboardLayer, element)
    if (renderedElement) {
      result.appendChild(renderedElement);
    }
  }
  return result;
}

function renderKeyboardLayoutElement(
    keyboardState: KeyboardState,
    keyboardLayer: KeyboardLayer,
    keyboardLayoutElement: KeyboardLayout | KeyLayout): HTMLElement | undefined {
  if (isKey(keyboardLayoutElement)) {
    return renderKey(keyboardState, keyboardLayer, keyboardLayoutElement);
  }
  if (isLayout(keyboardLayoutElement)) {
    if (keyboardLayoutElement.type === 'row') {
      return renderRowLayout(
        keyboardState, keyboardLayer, keyboardLayoutElement);
    } else if (keyboardLayoutElement.type === 'column') {
      return renderColumnLayout(
        keyboardState, keyboardLayer, keyboardLayoutElement);
    }
  }
}

function renderKeyboardLayoutLayers(
    keyboardState: KeyboardState,
    layers: { [key: string]: KeyboardLayout })
    : RenderedKeyboardLayout {
  const result: RenderedKeyboardLayout = {};
  for (const [layerName, keyboardLayout] of Object.entries(layers)) {
    const keyboardLayer = {
      ...keyboardLayout,
      active: false
    };
    const renderedKeyboardLayout = renderKeyboardLayoutElement(
      keyboardState, keyboardLayer, keyboardLayout);
    if (!renderedKeyboardLayout) {
      log('error', `Failed to render keyboard layout layer ${layerName}`);
      continue;
    }
    renderedKeyboardLayout.id = layerName;
    result[layerName] = {
      element: renderedKeyboardLayout,
      keyboardLayer: keyboardLayer,
    };
  }
  return result;
}

function enableLayer(layerName: string) {
  if (!layerName) {
    return;
  }
  const layer = document.getElementById(layerName);
  if (!layer) {
    log('error', `Layer by name ${layerName} not found`);
    return;
  }
  log('trace', `Enabling layer ${layerName}`);
  layer.classList.add('active');
}

function disableLayer(layerName: string) {
  if (!layerName) {
    return;
  }
  const layer = document.getElementById(layerName);
  if (!layer) {
    log('error', `Layer by name ${layerName} not found in DOM`);
    return;
  }
  log('trace', `Disabling layer ${layerName}`);
  layer.classList.remove('active');
}

function hideLayer(layerName: string) {
  if (!layerName) {
    return;
  }
  const layer = document.getElementById(layerName);
  if (!layer) {
    log('error', `Layer by name ${layerName} not found in DOM`);
    return;
  }
  log('trace', `Hiding layer ${layerName}`);
  layer.classList.add('transparent');
}

function unhideLayer(layerName: string) {
  if (!layerName) {
    return;
  }
  const layer = document.getElementById(layerName);
  if (!layer) {
    log('error', `Layer by name ${layerName} not found in DOM`);
    return;
  }
  log('debug', `Unhiding layer ${layerName}`);
  layer.classList.remove('transparent');
}

function handleHoveredKeys(
    config: Config,
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
    key.classList.add('cursor-hover');
  });
  [...releasedLeftKeys, ...releasedRightKeys].forEach((key) => {
    key.classList.remove('cursor-hover');
  });
  // if new keys are hovered/cursor moved to new key, send haptic feedback
  if (newHoveredLeftKeys.length != 0 && config?.cursor?.hapticOnHover) {
    triggerHapticPulse(1);
  }
  if (newHoveredRightKeys.length != 0 && config?.cursor?.hapticOnHover) {
    triggerHapticPulse(0);
  }
  keyboardState.hoveredLeftKeys = leftKeys;
  keyboardState.hoveredRightKeys = rightKeys;
}

function getKeys(x: number, y: number): KeyboardKey[] {
  log('trace', `Getting keys at { x: ${x}, y: ${y} }`);
  const elements = document.elementsFromPoint(x, y);
  const keys = elements.filter(element => element.nodeName.startsWith('KEYBOARD-KEY'));
  for (const key of keys) {
    if (key instanceof KeyboardKey
        && !key.isTrans()
        && key.isActive()) {
      log('trace', `Got key ${key.toString()}`);
    }
  }
  return keys
    .filter(key => key instanceof KeyboardKey)
    .filter(key => !key.isTrans())
    .filter(key => key.isActive())
    .reverse()
    .slice(0, 1);
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
 * @param config config information
 * @param keyboardState information about current state of the keyboard
 * @param input current input
 * @param lastInput input from last frame/tick
 * @param leftCursor left cursor for left touchpad
 * @param rightCursor right cursor for right touchpad
 */
async function handleTouchpads(
    config: Config,
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
  let leftKeys: KeyboardKey[] = [];
  if (lPadTouched) {
    leftKeys = getKeys(leftCursorX, leftCursorY);
  }
  let rightKeys: KeyboardKey[] = [];
  if (rPadTouched) {
    rightKeys = getKeys(rightCursorX, rightCursorY);
  }
  const now = new Date();
  handleHoveredKeys(config, keyboardState, leftKeys, rightKeys);
  if (lastInput.lPadForce < config.cursor.forceThreshold
      && input.lPadForce > config.cursor.forceThreshold) {
    if (config?.cursor?.hapticOnClick) {
      triggerHapticPulse(1);
    }
    keyboardState.keyStateChanges(
      leftKeys, 'down', now);
    keyboardState.pressedLeftKeys = leftKeys;
  } else if (lastInput.lPadForce > config.cursor.forceThreshold
      && input.lPadForce < config.cursor.forceThreshold) {
    if (config?.cursor?.hapticOnClick) {
      triggerHapticPulse(1);
    }
    keyboardState.keyStateChanges(
      keyboardState.pressedLeftKeys, 'up', now);
    keyboardState.pressedLeftKeys = [];
  }
  if (lastInput.rPadForce < config.cursor.forceThreshold
      && input.rPadForce > config.cursor.forceThreshold) {
    if (config?.cursor?.hapticOnClick) {
      triggerHapticPulse(0);
    }
    keyboardState.keyStateChanges(
      rightKeys, 'down', now);
    keyboardState.pressedRightKeys = rightKeys;
  } else if (lastInput.rPadForce > config.cursor.forceThreshold
      && input.rPadForce < config.cursor.forceThreshold) {
    if (config?.cursor?.hapticOnClick) {
      triggerHapticPulse(0);
    }
    keyboardState.keyStateChanges(
      keyboardState.pressedRightKeys, 'up', now);
    keyboardState.pressedRightKeys = [];
  }
}

class App {

  config: Config;
  keyboardState: KeyboardState;
  lastInput: SteamDeckDeviceReport | undefined;
  leftCursor: HTMLElement;
  rightCursor: HTMLElement;

  constructor() {
    this.config = {
      ...defaultConfig,
    }
    this.keyboardState = new KeyboardState();
    const leftCursor = document.querySelector<HTMLElement>('#leftCursor');
    const rightCursor = document.querySelector<HTMLElement>('#rightCursor');
    if (!leftCursor) {
      throw Error('Left cursor html element missing');
    }
    if (!rightCursor) {
      throw Error('Right cursor html element missing');
    }
    this.leftCursor = leftCursor;
    this.rightCursor = rightCursor;
  }

  async initListener() {
    await listen('config', this.onConfig.bind(this));
    await readConfig();
  }

  async onConfig(event: { payload: string }) {
    this.config = {
      ...this.config,
      ...JSON.parse(event?.payload),
    }
    const body = document.querySelector('body');
    this.keyboardState = new KeyboardState();
    const renderedKeyboardLayers = renderKeyboardLayoutLayers(
      this.keyboardState, this.config.layers);
    // add rendered keyboard layout to DOM
    Object.entries(renderedKeyboardLayers).reverse()
        .forEach(([layerName, renderedKeyboardLayout]) => {
      log('debug', `Adding layer ${layerName} to DOM`);
      this.keyboardState.keyboardLayers[layerName] =
        renderedKeyboardLayout.keyboardLayer;
      renderedKeyboardLayout.element.classList.add("keyboard-layout");
      body?.appendChild(renderedKeyboardLayout.element);
    });
    // enable first layer
    const firstLayerName = Object.keys(renderedKeyboardLayers)[0];
    this.keyboardState.enableLayer(firstLayerName); 
    await listen('input', this.onInput.bind(this));
  }

  async onInput(event: { payload: SteamDeckDeviceReport }) {
    if (!this.keyboardState) {
      return;
    }
    let input = event.payload;
    if (!this.lastInput) {
      this.lastInput = input;
      return;
    }
    handleTouchpads(
      this.config,
      this.keyboardState,
      input,
      this.lastInput,
      this.leftCursor,
      this.rightCursor);
    this.lastInput = input;
  }
}

window.addEventListener('DOMContentLoaded', () => {
  log('trace', 'DOM Content loaded event');
  const app = new App();
  app.initListener();
});
