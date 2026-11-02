import type { FileMeta } from './ipc';

export interface SmartName {
  title: string;
  detail?: string;
  skill: boolean;
}

/** Files whose meaning lives in their folder, not their literal name. */
export function smartName(file: FileMeta): SmartName {
  if (file.name.toLowerCase() === 'skill.md') {
    const parent = file.parent.split('/').filter(Boolean).pop();
    return {
      title: parent && parent.toLowerCase() !== 'skills' ? parent : 'Skill',
      detail: 'SKILL.md',
      skill: true,
    };
  }
  return { title: file.name, skill: false };
}
