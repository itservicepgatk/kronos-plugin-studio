import JSZip from 'jszip';
import { saveAs } from 'file-saver';
import { PluginProject } from '../types';

/**
 * Creates and triggers download of a .zip archive of the plugin project
 */
export async function downloadProjectAsZip(project: PluginProject): Promise<void> {
  const zip = new JSZip();

  for (const [path, content] of Object.entries(project.files)) {
    zip.file(path, content);
  }

  const blob = await zip.generateAsync({ type: 'blob' });
  const filename = `${project.name || 'kronos-plugin'}.zip`;
  saveAs(blob, filename);
}

/**
 * Reads a user-uploaded .zip file and reconstructs the PluginProject
 */
export async function loadProjectFromZip(file: File): Promise<PluginProject> {
  const zip = new JSZip();
  const loadedZip = await zip.loadAsync(file);
  const files: Record<string, string> = {};
  let pluginName = file.name.replace(/\.zip$/i, '');

  for (const [relativePath, zipEntry] of Object.entries(loadedZip.files)) {
    if (!zipEntry.dir) {
      const content = await zipEntry.async('string');
      files[relativePath] = content;

      if (relativePath.endsWith('Cargo.toml')) {
        const nameMatch = content.match(/name\s*=\s*["']([^"']+)["']/);
        if (nameMatch && nameMatch[1]) {
          pluginName = nameMatch[1];
        }
      }
    }
  }

  const fileKeys = Object.keys(files);
  const defaultActive = fileKeys.find(k => k.endsWith('lib.rs')) || fileKeys.find(k => k.endsWith('Cargo.toml')) || fileKeys[0] || '';

  return {
    name: pluginName,
    version: '0.1.0',
    description: `Loaded from ${file.name}`,
    files,
    activeFilePath: defaultActive,
  };
}
