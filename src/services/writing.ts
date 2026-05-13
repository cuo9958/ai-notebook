import { loggedInvoke } from '@/services/debug'
import type {
  WritingMaterial,
  WritingMaterialInput,
  WritingImageAsset,
  WritingProjectCreateInput,
  WritingProjectDetail,
  WritingProjectSaveInput,
  WritingProjectSummary,
  WritingPublishResult,
} from '@/types/writing'

export function listWritingProjects(): Promise<WritingProjectSummary[]> {
  return loggedInvoke('list_writing_projects')
}

export function createWritingProject(
  input?: WritingProjectCreateInput,
): Promise<WritingProjectDetail> {
  return loggedInvoke('create_writing_project', { input })
}

export function getWritingProject(projectId: string): Promise<WritingProjectDetail> {
  return loggedInvoke('get_writing_project', { projectId })
}

export function saveWritingProject(project: WritingProjectSaveInput): Promise<WritingProjectDetail> {
  return loggedInvoke('save_writing_project', { project }, { projectId: project.id, title: project.title })
}

export function saveWritingImage(
  projectId: string,
  fileName: string,
  data: number[],
): Promise<WritingImageAsset> {
  return loggedInvoke('save_writing_image', { projectId, fileName, data }, { projectId, fileName, dataLength: data.length })
}

export function deleteWritingProject(projectId: string): Promise<WritingProjectSummary[]> {
  return loggedInvoke('delete_writing_project', { projectId })
}

export function addWritingMaterial(
  projectId: string,
  material: WritingMaterialInput,
): Promise<WritingProjectDetail> {
  return loggedInvoke('add_writing_material', { projectId, material }, { projectId, materialKind: material.kind, title: material.title })
}

export function updateWritingMaterial(
  projectId: string,
  material: WritingMaterial,
): Promise<WritingProjectDetail> {
  return loggedInvoke('update_writing_material', {
    projectId,
    material: {
      id: material.id,
      kind: material.kind,
      title: material.title,
      content: material.content,
      sourceUrl: material.sourceUrl,
    },
  }, { projectId, materialId: material.id, materialKind: material.kind, title: material.title })
}

export function deleteWritingMaterial(
  projectId: string,
  materialId: string,
): Promise<WritingProjectDetail> {
  return loggedInvoke('delete_writing_material', { projectId, materialId })
}

export function publishWritingProject(
  projectId: string,
  directoryPath: string,
): Promise<WritingPublishResult> {
  return loggedInvoke('publish_writing_project', {
    input: {
      projectId,
      directoryPath,
    },
  })
}
