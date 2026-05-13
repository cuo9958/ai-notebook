export type WritingMaterialKind = 'text' | 'link'

export interface WritingProjectSummary {
  id: string
  title: string
  updatedAt: string
}

export interface WritingMaterial {
  id: string
  kind: WritingMaterialKind | string
  title: string
  content: string
  sourceUrl?: string | null
  createdAt: string
  updatedAt: string
}

export interface WritingOutline {
  id: string
  title: string
  content: string
}

export interface WritingSection {
  id: string
  title: string
  content: string
  selected: boolean
}

export interface WritingProjectDetail {
  id: string
  title: string
  publishDirectoryPath?: string | null
  updatedAt: string
  materials: WritingMaterial[]
  outlines: WritingOutline[]
  sections: WritingSection[]
}

export interface WritingProjectCreateInput {
  title?: string
}

export interface WritingOutlineInput {
  id?: string
  title: string
  content: string
}

export interface WritingSectionInput {
  id?: string
  title: string
  content: string
  selected: boolean
}

export interface WritingProjectSaveInput {
  id: string
  title: string
  publishDirectoryPath?: string | null
  outlines: WritingOutlineInput[]
  sections: WritingSectionInput[]
}

export interface WritingMaterialInput {
  kind: WritingMaterialKind | string
  title: string
  content: string
  sourceUrl?: string | null
}

export interface WritingPublishResult {
  notePath: string
  noteTitle: string
}

export interface WritingImageAsset {
  filePath: string
  markdownPath: string
}
