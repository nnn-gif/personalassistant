import { useEffect, useRef, useState, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

export enum StreamUpdateType {
  Thinking = 'thinking',
  StreamStart = 'streamStart',
  StreamDelta = 'streamDelta',
  StreamEnd = 'streamEnd',
  Error = 'error',
  SourcesFound = 'sourcesFound',
  Complete = 'complete',
}

export interface StreamUpdate {
  conversationId: string;
  messageId: string;
  updateType: StreamUpdateType;
  content?: string;
  delta?: string;
  metadata?: any;
}

export interface ThinkingMetadata {
  step: string;
  progress?: number;
}

export interface StreamingMessage {
  id: string;
  content: string;
  isComplete: boolean;
  isThinking: boolean;
  thinkingStep?: string;
  thinkingProgress?: number;
  sources?: Array<{
    document_id: string;
    score: number;
    preview: string;
  }>;
  error?: string;
  metadata?: any;
}

export function useStreamingChat() {
  const [streamingMessages, setStreamingMessages] = useState<Map<string, StreamingMessage>>(new Map());
  const unlistenRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    const setupListener = async () => {
      unlistenRef.current = await listen<StreamUpdate>('chat-stream', (event) => {
        const update = event.payload;
        
        setStreamingMessages((prev) => {
          const newMap = new Map(prev);
          const message = newMap.get(update.messageId) || {
            id: update.messageId,
            content: '',
            isComplete: false,
            isThinking: false,
          };

          switch (update.updateType) {
            case StreamUpdateType.Thinking:
              const thinkingMeta = update.metadata as ThinkingMetadata;
              newMap.set(update.messageId, {
                ...message,
                isThinking: true,
                thinkingStep: thinkingMeta?.step,
                thinkingProgress: thinkingMeta?.progress,
                metadata: update.metadata,
              });
              break;

            case StreamUpdateType.StreamStart:
              newMap.set(update.messageId, {
                ...message,
                isThinking: false,
                content: '',
              });
              break;

            case StreamUpdateType.StreamDelta:
              newMap.set(update.messageId, {
                ...message,
                content: update.content || message.content,
              });
              break;

            case StreamUpdateType.StreamEnd:
              newMap.set(update.messageId, {
                ...message,
                content: update.content || message.content,
              });
              break;

            case StreamUpdateType.SourcesFound:
              newMap.set(update.messageId, {
                ...message,
                sources: update.metadata?.sources,
              });
              break;

            case StreamUpdateType.Complete:
              newMap.set(update.messageId, {
                ...message,
                content: update.content || message.content,
                isComplete: true,
                isThinking: false,
              });
              break;

            case StreamUpdateType.Error:
              newMap.set(update.messageId, {
                ...message,
                error: update.metadata?.error,
                isComplete: true,
                isThinking: false,
              });
              break;
          }

          return newMap;
        });
      });
    };

    setupListener();

    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
      }
    };
  }, []);

  const sendStreamingMessage = useCallback(async (
    conversationId: string,
    messageId: string,
    message: string,
    mode: 'general' | 'knowledge',
    options?: {
      model?: string;
      goalId?: string;
      limit?: number;
    }
  ) => {
    // Initialize the streaming message
    setStreamingMessages((prev) => {
      const newMap = new Map(prev);
      newMap.set(messageId, {
        id: messageId,
        content: '',
        isComplete: false,
        isThinking: true,
        thinkingStep: 'Initializing',
      });
      return newMap;
    });

    try {
      if (mode === 'general') {
        await invoke('stream_general_chat', {
          conversationId,
          messageId,
          message,
          model: options?.model,
        });
      } else {
        await invoke('stream_document_chat', {
          conversationId,
          messageId,
          query: message,
          goalId: options?.goalId,
          limit: options?.limit,
          model: options?.model,
        });
      }
    } catch (error) {
      console.error('Failed to start streaming:', error);
      setStreamingMessages((prev) => {
        const newMap = new Map(prev);
        newMap.set(messageId, {
          id: messageId,
          content: '',
          isComplete: true,
          isThinking: false,
          error: error instanceof Error ? error.message : 'Unknown error',
        });
        return newMap;
      });
      throw error;
    }
  }, []);

  const getStreamingMessage = useCallback((messageId: string): StreamingMessage | undefined => {
    return streamingMessages.get(messageId);
  }, [streamingMessages]);

  const clearStreamingMessage = useCallback((messageId: string) => {
    setStreamingMessages((prev) => {
      const newMap = new Map(prev);
      newMap.delete(messageId);
      return newMap;
    });
  }, []);

  return {
    sendStreamingMessage,
    getStreamingMessage,
    clearStreamingMessage,
    streamingMessages,
  };
}