import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Brain, Search, FileText, Sparkles } from 'lucide-react';

interface ThinkingIndicatorProps {
  step?: string;
  progress?: number;
  isVisible: boolean;
  thinkingContent?: string;
}

export const ThinkingIndicator: React.FC<ThinkingIndicatorProps> = ({
  step = 'Thinking',
  progress,
  isVisible,
  thinkingContent,
}) => {
  const getIcon = () => {
    if (step.toLowerCase().includes('search')) return Search;
    if (step.toLowerCase().includes('document')) return FileText;
    if (step.toLowerCase().includes('generat')) return Sparkles;
    return Brain;
  };

  const Icon = getIcon();

  return (
    <AnimatePresence>
      {isVisible && thinkingContent && (
        <motion.div
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: 'auto' }}
          exit={{ opacity: 0, height: 0 }}
          transition={{ duration: 0.3 }}
          className="p-3 bg-gray-800/50 rounded-md border border-gray-700"
        >
          <p className="text-xs font-mono text-gray-400 whitespace-pre-wrap">
            {thinkingContent}
          </p>
        </motion.div>
      )}
    </AnimatePresence>
  );
};