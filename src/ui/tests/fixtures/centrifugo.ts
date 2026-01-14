import { APIRequestContext, request } from '@playwright/test';

export async function publishToCentrifugo(channel: string, data: any) {
  const context = await request.newContext({
    ignoreHTTPSErrors: true,
  });
  const response = await context.post('https://localhost:8000/api', {
    headers: {
      'Authorization': 'apikey api_key',
      'Content-Type': 'application/json',
    },
    data: {
      method: 'publish',
      params: {
        channel,
        data,
      },
    },
  });

  if (!response.ok()) {
    console.error(`Failed to publish to Centrifugo: ${response.status()} ${response.statusText()}`);
    console.error(await response.text());
    throw new Error('Centrifugo publish failed');
  }
}
