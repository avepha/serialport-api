import { expect, Locator, Page, test } from '@playwright/test';
import {
  commandPayload,
  commandPostCount,
  connectionFixture,
  expectRequestBody,
  installDashboardApiMocks,
  latestRequest,
  portFixture,
  presetDeleteCount,
  presetFixture,
} from './fixtures';

test.beforeEach(async ({ page }) => {
  page.on('pageerror', (error) => {
    throw error;
  });
  page.on('console', (message) => {
    if (message.type() === 'error' && message.text().startsWith('Failed to load resource: the server responded with a status of 500')) {
      return;
    }
    if (['error', 'warning'].includes(message.type())) {
      throw new Error(`Unexpected browser console ${message.type()}: ${message.text()}`);
    }
  });
});

test('renders the empty hardware-free dashboard state with mocked APIs', async ({ page }) => {
  await installDashboardApiMocks(page);

  await page.goto('/');

  await expect(page.getByRole('heading', { name: /Serial control dashboard/i })).toBeVisible();
  await expect(page.getByText('ok · v0.1.0')).toBeVisible();
  await expect(page.getByText('memory').first()).toBeVisible();
  await expect(page.getByText('0 visible')).toBeVisible();
  await expect(page.getByText('connected')).toBeVisible();
  await expect(page.getByText('No active connections. Create one to enable commands.')).toBeVisible();
  await expect(page.getByText('No ports reported by the API.')).toBeVisible();
  await expect(page.getByText('Dashboard data refreshed')).toBeVisible();
});

test('renders populated fixture data for ports, connections, presets, and command target', async ({ page }) => {
  await installDashboardApiMocks(page, {
    ports: [portFixture],
    connections: [connectionFixture],
    presets: [presetFixture],
  });

  await page.goto('/dashboard');

  await expect(page.getByRole('heading', { name: /Serial control dashboard/i })).toBeVisible();
  await expect(cardByTitle(page, 'Connections').getByRole('cell', { name: 'default' })).toBeVisible();
  await expect(page.getByText('/dev/ROBOT')).toBeVisible();
  await expect(cardByTitle(page, 'Connections').getByText('connected')).toBeVisible();
  await expect(page.getByRole('combobox')).toContainText('default');
  await expect(page.getByText('/dev/ttyUSB0')).toBeVisible();
  await expect(page.getByText('Test Adapter')).toBeVisible();

  await page.getByRole('tab', { name: 'Presets' }).click();
  await expect(page.getByText('Read sensor')).toBeVisible();
  await expect(page.getByText('sensor.read')).toBeVisible();
});

test('connect flow posts decoded delimiter and refreshes the connection list', async ({ page }) => {
  const state = await installDashboardApiMocks(page);

  await page.goto('/');
  await expect(page.getByText('No active connections. Create one to enable commands.')).toBeVisible();
  await cardByTitle(page, 'Connections').getByRole('button', { name: 'Connect' }).click();

  await expectRequestBody(state, 'POST', '/api/v1/connections', {
    name: 'default',
    port: '/dev/ROBOT',
    baudRate: 115200,
    delimiter: '\r\n',
  });
  await expect(cardByTitle(page, 'Connections').getByText('/dev/ROBOT')).toBeVisible();
  await expect(cardByTitle(page, 'Connections').getByText('connected')).toBeVisible();
});

test('send command posts parsed JSON to the selected connection', async ({ page }) => {
  const state = await installDashboardApiMocks(page, { connections: [connectionFixture] });

  await page.goto('/');
  await expect(cardByTitle(page, 'Connections').getByRole('cell', { name: 'default' })).toBeVisible();
  await page.getByRole('button', { name: 'Send command' }).click();

  await expectRequestBody(state, 'POST', '/api/v1/connections/default/commands', {
    payload: commandPayload,
    waitForResponse: false,
  });
  await expect(page.getByText('Command 1 queued')).toBeVisible();
});

test('invalid JSON payload fails locally without sending a command request', async ({ page }) => {
  const state = await installDashboardApiMocks(page, { connections: [connectionFixture] });

  await page.goto('/');
  await page.getByLabel('Command JSON payload').fill('[]');
  await page.getByRole('button', { name: 'Send command' }).click();

  await expect(page.getByText('Action failed')).toBeVisible();
  await expect(page.getByText('Payload must be a JSON object')).toBeVisible();
  expect(commandPostCount(state)).toBe(0);
  expect(latestRequest(state, 'POST', '/api/v1/connections/default/commands')).toBeUndefined();
});

test('preset create, load, and delete use mocked API state transitions', async ({ page }) => {
  const state = await installDashboardApiMocks(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Save as preset' }).click();

  await expectRequestBody(state, 'POST', '/api/v1/presets', {
    name: 'Read sensor',
    payload: commandPayload,
  });

  await page.getByRole('tab', { name: 'Presets' }).click();
  await expect(page.getByText('Read sensor')).toBeVisible();
  await expect(page.getByText('sensor.read')).toBeVisible();

  await page.getByRole('button', { name: 'Load' }).click();
  await expect(page.getByText('Loaded preset Read sensor')).toBeVisible();
  await page.getByRole('tab', { name: 'Control' }).click();
  await expect(page.getByLabel('Command JSON payload')).toHaveValue(JSON.stringify(commandPayload, null, 2));

  await page.getByRole('tab', { name: 'Presets' }).click();
  await page.getByRole('button', { name: 'Delete preset Read sensor' }).click();
  await expect.poll(() => presetDeleteCount(state)).toBe(1);
  await expect(page.getByText('No presets saved.')).toBeVisible();
});

test('API failure renders a visible notice without crashing the dashboard', async ({ page }) => {
  await installDashboardApiMocks(page, { failures: { '/api/v1/status': 500 } });

  await page.goto('/');

  await expect(page.getByRole('heading', { name: /Serial control dashboard/i })).toBeVisible();
  await expect(page.getByText('Action failed')).toBeVisible();
  await expect(page.getByText('Status unavailable: 500 Internal Server Error: simulated failure')).toBeVisible();
  await expect(page.getByText('No active connections. Create one to enable commands.')).toBeVisible();
});

test('events tab shows a deterministic mocked EventSource event', async ({ page }) => {
  await installDashboardApiMocks(page, {
    event: { name: 'serial.log', data: { level: 'info', message: 'mock stream opened' } },
  });

  await page.goto('/');
  await page.getByRole('tab', { name: 'Events' }).click();

  await expect(page.getByText('serial.log').first()).toBeVisible();
  await expect(page.getByText('mock stream opened').first()).toBeVisible();
});

function cardByTitle(page: Page, title: string): Locator {
  return page.getByRole('heading', { name: title }).locator('xpath=ancestor::div[contains(@class, "rounded-xl")][1]');
}
