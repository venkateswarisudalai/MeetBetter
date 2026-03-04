export async function generateSummary(
  apiKey: string,
  transcript: string,
  meetingContext?: string
): Promise<string> {
  const systemPrompt = `You are an expert meeting assistant. Analyze the following meeting transcript and provide a structured summary.

Format your response as:
## Key Points
- (bullet points of main discussion topics)

## Action Items
- (specific tasks mentioned, with owners if stated)

## Decisions Made
- (any decisions reached during the meeting)

## Summary
(2-3 sentence overview of the meeting)

${meetingContext ? `Meeting Context: ${meetingContext}` : ''}`;

  const response = await fetch('https://api.groq.com/openai/v1/chat/completions', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      model: 'llama-3.3-70b-versatile',
      messages: [
        { role: 'system', content: systemPrompt },
        { role: 'user', content: `Here is the meeting transcript:\n\n${transcript}` },
      ],
      temperature: 0.3,
      max_tokens: 2000,
    }),
  });

  if (!response.ok) {
    const err = await response.text();
    throw new Error(`Groq API error: ${response.status} - ${err}`);
  }

  const data = await response.json();
  return data.choices[0]?.message?.content || 'No summary generated.';
}

export async function askAboutMeeting(
  apiKey: string,
  transcript: string,
  question: string
): Promise<string> {
  const response = await fetch('https://api.groq.com/openai/v1/chat/completions', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      model: 'llama-3.3-70b-versatile',
      messages: [
        {
          role: 'system',
          content: 'You are a helpful meeting assistant. Answer questions based on the meeting transcript provided. Be concise and specific.',
        },
        {
          role: 'user',
          content: `Meeting transcript:\n${transcript}\n\nQuestion: ${question}`,
        },
      ],
      temperature: 0.3,
      max_tokens: 1000,
    }),
  });

  if (!response.ok) {
    throw new Error(`Groq API error: ${response.status}`);
  }

  const data = await response.json();
  return data.choices[0]?.message?.content || 'Could not generate answer.';
}
