import type { Board, BoardOutlineScene, OutlineFeature, ProjectDoc } from '@boardstudio/v2-contracts';

export function outlineVersion(document: ProjectDoc, boardId: string) {
  const state = document.boardOutlines?.find(item => item.boardId === boardId);
  return state?.versions.find(version => version.id === state.activeVersionId);
}
export function nextOutlineName(document: ProjectDoc, boardId: string): string {
  const versions = document.boardOutlines?.find(item => item.boardId === boardId)?.versions ?? [];
  let number = 1;
  while (versions.some(version => version.name === `Edited outline ${number}`)) number += 1;
  return `Edited outline ${number}`;
}

export function editableOutlineFeatures(document: ProjectDoc, board: Board, scene?: BoardOutlineScene): OutlineFeature[] {
  const version = outlineVersion(document, board.id);
  if (version) return version.geometry.features;
  return scene?.sourceContours.map((contour, index) => ({
    id: `outline-source:${board.id}:${index}`, kind: 'polygon', points: contour.points,
    operation: contour.hole ? 'subtract' : 'add',
  })) ?? [];
}

export function updateOutlineFeature(document: ProjectDoc, board: Board, feature: OutlineFeature): ProjectDoc {
  const version = outlineVersion(document, board.id);
  if (version?.geometry.features.some(item => item.id === feature.id)) return {
    ...document, boardOutlines: document.boardOutlines?.map(state => state.boardId !== board.id ? state : {
      ...state, versions: state.versions.map(item => item.id !== version.id ? item : {
        ...item, geometry: { ...item.geometry, features: item.geometry.features.map(item => item.id === feature.id ? feature : item) },
      }),
    }),
  };
  return { ...document, outline: document.outline.map(item => item.id === feature.id ? feature : item) };
}
