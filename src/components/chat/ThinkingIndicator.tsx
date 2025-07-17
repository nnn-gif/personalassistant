import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';

interface ThinkingIndicatorProps {
  step?: string;
  isVisible: boolean;
  thinkingContent?: string;
}

export const ThinkingIndicator: React.FC<ThinkingIndicatorProps> = ({
  isVisible,
  thinkingContent,
}) => {

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