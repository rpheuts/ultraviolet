document.addEventListener('DOMContentLoaded', () => {
    // DOM Elements
    const chatContainer = document.getElementById('chat-container');
    const fileDropArea = document.getElementById('file-drop-area');
    const fileInput = document.getElementById('file-input');
    const filesList = document.getElementById('files-list');
    const promptInput = document.getElementById('prompt-input');
    const sendButton = document.getElementById('send-button');
    const shutdownButton = document.getElementById('shutdown-button');
    
    // Code highlighting
    let codeHighlightInterval = null;

    // State
    let wsConnection = null;
    let files = [];  // Persistent file list
    let conversationHistory = [];  // Track conversation history
    
    // Find code blocks in text
    function findCodeBlocks(text) {
        // Regex to match markdown code blocks with language specifiers
        const codeBlockRegex = /```([a-z]*)\n([\s\S]*?)```/g;
        return [...text.matchAll(codeBlockRegex)];
    }
    
    // Process content for code blocks and apply syntax highlighting
    function processCodeBlocks(element) {
        if (!element) return;
        
        const content = element.textContent;
        const codeBlocks = findCodeBlocks(content);
        
        if (codeBlocks.length === 0) return;
        
        // Create a new HTML structure with highlighted code
        let lastIndex = 0;
        let newHtml = '';
        
        for (const match of codeBlocks) {
            const [fullMatch, language, code] = match;
            const matchIndex = match.index;
            
            // Add text before this code block
            newHtml += content.substring(lastIndex, matchIndex);
            
            // Add highlighted code block
            try {
                let highlightedCode;
                if (language) {
                    highlightedCode = hljs.highlight(code, {language}).value;
                } else {
                    highlightedCode = hljs.highlightAuto(code).value;
                }
                
                newHtml += `<pre data-language="${language || 'code'}"><code class="language-${language || 'plaintext'}">${highlightedCode}</code></pre>`;
            } catch (error) {
                console.error('Error highlighting code:', error);
                // Fallback to plain code block
                newHtml += `<pre><code>${code}</code></pre>`;
            }
            
            lastIndex = matchIndex + fullMatch.length;
        }
        
        // Add remaining text after the last code block
        newHtml += content.substring(lastIndex);
        
        // Update the element's content
        element.innerHTML = newHtml;
    }
    
    // Start periodic code highlighting
    function startCodeHighlighting(element) {
        // Clear any existing interval
        if (codeHighlightInterval) {
            clearInterval(codeHighlightInterval);
        }
        
        // Set up new interval
        codeHighlightInterval = setInterval(() => {
            processCodeBlocks(element);
        }, 500); // Check every 500ms
    }
    
    // Stop periodic code highlighting
    function stopCodeHighlighting() {
        if (codeHighlightInterval) {
            clearInterval(codeHighlightInterval);
            codeHighlightInterval = null;
        }
    }
    
    // Connect to WebSocket
    function connectWebSocket() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/ws`;
        
        console.log(`Connecting to WebSocket at ${wsUrl}`);
        wsConnection = new WebSocket(wsUrl);
        
        wsConnection.onopen = () => {
            console.log('WebSocket connection established');
            sendButton.disabled = false;
        };
        
        wsConnection.onclose = () => {
            console.log('WebSocket connection closed');
            sendButton.disabled = true;
            // Try to reconnect after a delay
            setTimeout(connectWebSocket, 3000);
        };
        
        wsConnection.onerror = (error) => {
            console.error('WebSocket error:', error);
        };
        
        wsConnection.onmessage = (event) => {
            const message = event.data;
            
            // Check if this is the end of the message
            if (message === '\n[DONE]') {
                console.log("End of message received");
                
                // Remove typing indicator and streaming class
                const typingIndicator = document.querySelector('.typing-indicator');
                const streamingMessage = document.querySelector('.ai-message.streaming');
                
                if (typingIndicator) {
                    typingIndicator.remove();
                }
                
                if (streamingMessage) {
                    // Final code highlighting pass
                    processCodeBlocks(streamingMessage);
                    
                    // Stop the periodic highlighting
                    stopCodeHighlighting();
                    
                    streamingMessage.classList.remove('streaming');
                    
                    // Store AI response in conversation history
                    if (conversationHistory.length > 0) {
                        conversationHistory[conversationHistory.length - 1].ai = 
                            streamingMessage.textContent;
                    }
                }
                return;
            }
            
            // If this is the first chunk, create a new message element
            let aiMessage = document.querySelector('.ai-message.streaming');
            
            if (!aiMessage) {
                // Create new message element
                aiMessage = document.createElement('div');
                aiMessage.className = 'message ai-message streaming';
                chatContainer.appendChild(aiMessage);
                
                // Start periodic code highlighting
                startCodeHighlighting(aiMessage);
            }
            
            // Simply append the message text directly
            aiMessage.textContent += message;
            
            // Scroll to bottom with requestAnimationFrame to ensure rendering completes first
            window.requestAnimationFrame(() => {
                chatContainer.scrollTop = chatContainer.scrollHeight;
            });
        };
    }
    
    // Initialize WebSocket connection
    connectWebSocket();
    
    // Format prompt with conversation history
    function formatPromptWithHistory(newPrompt) {
        // If this is the first message, just return it
        if (conversationHistory.length === 0) {
            return newPrompt;
        }
        
        // Format conversation history
        let formattedPrompt = "Previous conversation:\n\n";
        
        // Add previous exchanges
        conversationHistory.forEach(turn => {
            formattedPrompt += `USER: ${turn.user}\n`;
            
            // Add file information if any were included
            if (turn.files && turn.files.length > 0) {
                const fileNames = turn.files.map(path => 
                    path.split('/').pop().split('-').slice(1).join('-'));
                formattedPrompt += `[User provided ${fileNames.length} file(s): ${fileNames.join(', ')}]\n`;
            }
            
            formattedPrompt += `AI: ${turn.ai}\n\n`;
        });
        
        // Add the current prompt
        formattedPrompt += `USER: ${newPrompt}\n`;
        
        // Add current file information if any
        if (files.length > 0) {
            const fileNames = files.map(path => 
                path.split('/').pop().split('-').slice(1).join('-'));
            formattedPrompt += `[User provided ${fileNames.length} file(s): ${fileNames.join(', ')}]`;
        }
        
        return formattedPrompt;
    }
    
    // Create reset conversation button
    function createResetButton() {
        // Check if button already exists
        if (document.getElementById('reset-conversation')) {
            return;
        }
        
        const headerContainer = document.querySelector('.header-container');
        
        const resetButton = document.createElement('button');
        resetButton.id = 'reset-conversation';
        resetButton.className = 'reset-button';
        resetButton.textContent = 'New Conversation';
        
        resetButton.addEventListener('click', () => {
            if (confirm('Start a new conversation? This will clear the current chat history.')) {
                // Clear conversation history
                conversationHistory = [];
                
                // Clear chat UI except welcome message
                chatContainer.innerHTML = '';
                
                // Re-add welcome message
                const welcomeMessage = document.createElement('div');
                welcomeMessage.className = 'welcome-message';
                welcomeMessage.innerHTML = '<h2>Welcome to Blue AI Chat</h2><p>Ask a question or upload files for context.</p>';
                chatContainer.appendChild(welcomeMessage);
                
                // Note: We're not clearing files, as they might still be useful in the new conversation
            }
        });
        
        // Insert as second child (after the h1)
        headerContainer.insertBefore(resetButton, headerContainer.children[1]);
    }
    
    // Create reset button on page load
    createResetButton();
    
    // Send prompt to AI
    function sendPrompt() {
        const prompt = promptInput.value.trim();
        if (!prompt || !wsConnection || wsConnection.readyState !== WebSocket.OPEN) {
            return;
        }
        
        // Remove welcome message if present
        const welcomeMessage = document.querySelector('.welcome-message');
        if (welcomeMessage) {
            welcomeMessage.remove();
        }
        
        // Add user message to chat
        const userMessage = document.createElement('div');
        userMessage.className = 'message user-message';
        userMessage.textContent = prompt;
        chatContainer.appendChild(userMessage);
        
        // Add typing indicator
        const aiMessage = document.createElement('div');
        aiMessage.className = 'message ai-message streaming'; // Add streaming class to reuse this element
        
        const typingIndicator = document.createElement('div');
        typingIndicator.className = 'typing-indicator';
        typingIndicator.innerHTML = '<span></span><span></span><span></span>';
        
        aiMessage.appendChild(typingIndicator);
        chatContainer.appendChild(aiMessage);
        
        // Scroll to bottom with requestAnimationFrame to ensure rendering completes first
        window.requestAnimationFrame(() => {
            chatContainer.scrollTop = chatContainer.scrollHeight;
        });
        
        // Format prompt with conversation history
        const formattedPrompt = formatPromptWithHistory(prompt);
        
        // Add to conversation history
        conversationHistory.push({
            user: prompt,
            ai: "",  // Will be filled when AI responds
            files: [...files]  // Store copy of current files
        });
        
        // Send message to WebSocket
        const message = JSON.stringify({
            prompt: formattedPrompt,
            files: files
        });
        
        console.log('Sending message to WebSocket');
        wsConnection.send(message);
        
        // Clear input
        promptInput.value = '';
    }
    
    // File upload handling
    function uploadFiles(fileList) {
        if (!fileList || fileList.length === 0) return;
        
        const formData = new FormData();
        for (let i = 0; i < fileList.length; i++) {
            formData.append('files', fileList[i]);
        }
        
        fetch('/api/upload', {
            method: 'POST',
            body: formData
        })
        .then(response => response.json())
        .then(data => {
            if (data.file_paths && data.file_paths.length > 0) {
                // Add to persistent files list
                files = files.concat(data.file_paths);
                updateFilesList();
            }
        })
        .catch(error => {
            console.error('Error uploading files:', error);
        });
    }
    
    // Update files list UI with clear all button
    function updateFilesList() {
        filesList.innerHTML = '';
        
        if (files.length === 0) {
            return;
        }
        
        // Add header
        const header = document.createElement('div');
        header.className = 'files-header';
        header.textContent = 'Files in conversation:';
        filesList.appendChild(header);
        
        // Create file list container
        const filesContainer = document.createElement('div');
        filesContainer.className = 'files-container';
        
        files.forEach((filePath, index) => {
            const fileName = filePath.split('/').pop().split('-').slice(1).join('-');
            
            const fileItem = document.createElement('div');
            fileItem.className = 'file-item';
            
            const fileNameSpan = document.createElement('span');
            fileNameSpan.className = 'file-name';
            fileNameSpan.textContent = fileName;
            fileItem.appendChild(fileNameSpan);
            
            const removeButton = document.createElement('span');
            removeButton.className = 'remove-file';
            removeButton.textContent = '×';
            removeButton.addEventListener('click', () => {
                files.splice(index, 1);
                updateFilesList();
            });
            fileItem.appendChild(removeButton);
            
            filesContainer.appendChild(fileItem);
        });
        
        filesList.appendChild(filesContainer);
        
        // Add clear all button if multiple files
        if (files.length > 1) {
            const clearAllButton = document.createElement('button');
            clearAllButton.className = 'clear-files-button';
            clearAllButton.textContent = 'Remove all files';
            clearAllButton.addEventListener('click', () => {
                if (confirm('Remove all files from the conversation?')) {
                    files = [];
                    updateFilesList();
                }
            });
            filesList.appendChild(clearAllButton);
        }
    }
    
    // File drag and drop handlers
    ['dragenter', 'dragover', 'dragleave', 'drop'].forEach(eventName => {
        fileDropArea.addEventListener(eventName, (e) => {
            e.preventDefault();
            e.stopPropagation();
        }, false);
    });
    
    ['dragenter', 'dragover'].forEach(eventName => {
        fileDropArea.addEventListener(eventName, () => {
            fileDropArea.classList.add('active');
        }, false);
    });
    
    ['dragleave', 'drop'].forEach(eventName => {
        fileDropArea.addEventListener(eventName, () => {
            fileDropArea.classList.remove('active');
        }, false);
    });
    
    fileDropArea.addEventListener('drop', (e) => {
        const droppedFiles = e.dataTransfer.files;
        uploadFiles(droppedFiles);
    }, false);
    
    // Click to upload
    fileDropArea.addEventListener('click', () => {
        fileInput.click();
    });
    
    fileInput.addEventListener('change', () => {
        uploadFiles(fileInput.files);
        fileInput.value = ''; // Reset file input
    });
    
    // Send button click event
    sendButton.addEventListener('click', sendPrompt);
    
    // Send on Enter key (but allow shift+enter for new lines)
    promptInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            sendPrompt();
        }
    });
    
    // Shutdown server handler
    shutdownButton.addEventListener('click', () => {
        if (confirm('Are you sure you want to shut down the server?')) {
            // Add system message
            const systemMessage = document.createElement('div');
            systemMessage.className = 'message system-message';
            systemMessage.textContent = 'Shutting down server...';
            chatContainer.appendChild(systemMessage);
            
            // Scroll to bottom with requestAnimationFrame to ensure rendering completes first
            window.requestAnimationFrame(() => {
                chatContainer.scrollTop = chatContainer.scrollHeight;
            });
            
            // Disable the button to prevent multiple clicks
            shutdownButton.disabled = true;
            shutdownButton.textContent = 'Shutting down...';
            
            // Call shutdown API
            fetch('/api/shutdown', {
                method: 'POST'
            })
            .then(() => {
                console.log('Server shutdown initiated');
                
                // Update the message
                systemMessage.textContent = 'Server shutdown initiated. You can close this window.';
                
                // Disable the send button
                sendButton.disabled = true;
            })
            .catch(error => {
                console.error('Error shutting down server:', error);
                systemMessage.textContent = 'Failed to shut down server. Please try again.';
                shutdownButton.disabled = false;
                shutdownButton.textContent = 'Shutdown Server';
            });
        }
    });
});
