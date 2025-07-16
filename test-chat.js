// Test script to verify chat commands
import { invoke } from '@tauri-apps/api/core';

async function testChatCommands() {
  console.log('Testing chat commands...');
  
  try {
    // Test list_ollama_models
    console.log('\n1. Testing list_ollama_models...');
    const models = await invoke('list_ollama_models');
    console.log('Available models:', models);
    
    // Test chat_general
    console.log('\n2. Testing chat_general...');
    const generalResponse = await invoke('chat_general', {
      message: 'Hello, how are you?',
      goalId: null,
      model: 'llama3.2:1b'
    });
    console.log('General chat response:', generalResponse);
    
    // Test chat_with_knowledge (without documents indexed)
    console.log('\n3. Testing chat_with_knowledge...');
    const knowledgeResponse = await invoke('chat_with_knowledge', {
      message: 'What documents do I have?',
      goalId: null,
      model: 'llama3.2:1b'
    });
    console.log('Knowledge chat response:', knowledgeResponse);
    
    console.log('\nAll tests completed successfully!');
  } catch (error) {
    console.error('Error during testing:', error);
  }
}

// Run tests when the script loads
testChatCommands();