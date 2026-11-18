import React from 'react';
import { Layers, X, ArrowRight } from 'lucide-react';
import { STARTER_TEMPLATES } from '../constants/templates';
import { PluginTemplate } from '../types';

interface TemplateModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelectTemplate: (template: PluginTemplate) => void;
}

export const TemplateModal: React.FC<TemplateModalProps> = ({
  isOpen,
  onClose,
  onSelectTemplate,
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-[#181820] border border-[#2d2d38] w-full max-w-2xl rounded-2xl shadow-2xl p-5 text-gray-200">
        <div className="flex items-center justify-between pb-3 border-b border-[#282836]">
          <div className="flex items-center gap-2">
            <Layers className="w-5 h-5 text-amber-400" />
            <h2 className="font-semibold text-sm">Choose a Plugin Template</h2>
          </div>
          <button onClick={onClose} className="p-1 text-gray-400 hover:text-gray-200 rounded">
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="mt-4 grid grid-cols-1 md:grid-cols-2 gap-3 max-h-[60vh] overflow-y-auto p-1">
          {STARTER_TEMPLATES.map((tmpl) => (
            <div
              key={tmpl.id}
              onClick={() => {
                onSelectTemplate(tmpl);
                onClose();
              }}
              className="bg-[#131317] hover:bg-[#1b1b22] border border-[#2a2a36] hover:border-amber-500/50 p-4 rounded-xl cursor-pointer transition-all flex flex-col justify-between group"
            >
              <div>
                <div className="flex items-center justify-between">
                  <h3 className="font-semibold text-sm text-gray-100 group-hover:text-amber-400 transition-colors">
                    {tmpl.title}
                  </h3>
                  <ArrowRight className="w-4 h-4 text-gray-500 group-hover:text-amber-400 group-hover:translate-x-0.5 transition-all" />
                </div>
                <p className="text-xs text-gray-400 mt-2 leading-relaxed">
                  {tmpl.description}
                </p>
              </div>

              <div className="flex flex-wrap gap-1.5 mt-3 pt-2 border-t border-[#23232e]">
                {tmpl.tags.map((tag) => (
                  <span
                    key={tag}
                    className="text-[10px] font-mono px-2 py-0.5 rounded bg-[#20202a] text-gray-400"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </div>
          ))}
        </div>

        <div className="flex justify-end pt-3 border-t border-[#282836] mt-4">
          <button
            onClick={onClose}
            className="px-3 py-1.5 text-xs text-gray-400 hover:text-gray-200"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
