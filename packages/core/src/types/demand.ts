export interface UserVO {
  erp: string;
  name: string;
  orgId?: number;
  orgName?: string;
}

export interface CardAttachment {
  tagName: string;
  name: string;
  url: string;
}

export interface DemandMetadata {
  demandId: number;
  demandCode: string;
  name: string;
  status?: number;
  demandProjectId?: number;
  processor?: UserVO;
  proposer?: UserVO;
  receiver?: UserVO;
  prdUrl: string;
  attachments: CardAttachment[];
  rawLink: string;
}
