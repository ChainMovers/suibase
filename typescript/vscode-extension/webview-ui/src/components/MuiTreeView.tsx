import * as React from 'react';
import { styled } from '@mui/material/styles';

import { RichTreeView } from '@mui/x-tree-view/RichTreeView';
import { TreeViewBaseItem } from '@mui/x-tree-view/models';
import Box from '@mui/material/Box';

//import { css } from '@emotion/react';

import {
  unstable_useTreeItem2 as useTreeItem2,
  UseTreeItem2Parameters,
} from '@mui/x-tree-view/useTreeItem2';
import {
  TreeItem2Content,
  TreeItem2IconContainer,
  TreeItem2GroupTransition,  
  TreeItem2Root,
} from '@mui/x-tree-view/TreeItem2';
import { TreeItem2Icon } from '@mui/x-tree-view/TreeItem2Icon';
import { TreeItem2Provider } from '@mui/x-tree-view/TreeItem2Provider';
import { Typography } from '@mui/material';
import { TREE_ID_INSERT_ADDR, TREE_ITEM_EMPTY, TREE_ITEM_ID_RECENT_PACKAGES_EMPTY, TREE_ITEM_PACKAGE } from '../common/Consts';
import CopyToClipboardButton from './CopyToClipboardButton';

const CustomTreeItemContent = styled(TreeItem2Content)(({ }) => ({
  padding: 0,
}));


function shortAddr( addr: string ): string {
  // Remove a leading "0x" if present in addr.  
  addr = addr.trim();
  addr = addr.replace(/^0x/, "");  

  // Given a string, take the first two character of addr, append a "~" and then append the last 3 character of addr.
  // Example: "1234567890" => "12~890"
  return "0x" + (addr.length > 5 ? addr.slice(0,2) + "~" + addr.slice(-3) : addr);
}

interface CustomTreeItemProps
  extends Omit<UseTreeItem2Parameters, 'rootRef'>,
    Omit<React.HTMLAttributes<HTMLLIElement>, 'onFocus'> 
{
  workdir: string;
}

// Apply styles to some inner parts of the label (e.g. "0x")
// Intended as the last minor styling "touch-up" done prior to rendering.
const labelInnerStyling = (label: React.ReactNode) => {
  const str = String(label);
  const parts = str.split('0x');

  return parts.map((part, i) => (
    <React.Fragment key={i}>
      {i > 0 && <span style={{fontSize: "9px", filter: 'brightness(50%)', fontWeight: 'lighter'}}>0x</span>}
      {part}
    </React.Fragment>
  ));
};

const CustomTreeItem = React.forwardRef(function CustomTreeItem(
  props: CustomTreeItemProps,
  ref: React.Ref<HTMLLIElement>,
) {
  const { workdir, id, itemId, disabled, children, ...other } = props;
  let { label } = props;

  // set is_top_folder to true if first character of itemId is a numeric.
  // See Consts.ts for the meaning of the first char ( TREE_ITEM_x ).
  let is_top_folder = false;
  let is_empty_folder = false;
  let empty_folder_label = '(empty)';
  let to_clipboard: string | undefined = undefined;
  const first_char = itemId.charAt(0);
  if (first_char.length > 0) {
    if (first_char >= '0' && first_char <= '9') {
        is_top_folder = true;
    } else if (first_char === TREE_ITEM_EMPTY) {
        is_empty_folder = true;
        if (itemId === TREE_ITEM_ID_RECENT_PACKAGES_EMPTY) {
          empty_folder_label = `To get started, do '${workdir} publish' in a terminal`;
        }
    } else if (first_char === TREE_ITEM_PACKAGE) {
      // Extract the packageId from the id (all character after last "-").      
      // In the label, replace TREE_ID_INSERT_ADDR with the packageId.      
      const packageId = itemId.split('-').pop();
      if (label && packageId) {
        const shortPackageId = shortAddr(packageId);
        label = label.toString().replace(TREE_ID_INSERT_ADDR, shortPackageId);
        to_clipboard = packageId;
      }
    }
  }


  const {
    getRootProps,
    getContentProps,
    getIconContainerProps,
    getLabelProps,
    getGroupTransitionProps,
    status,
  } = useTreeItem2({ id, itemId, children, label, disabled, rootRef: ref });

  let labelStyle: React.CSSProperties = { padding: 0, whiteSpace: 'nowrap', fontSize:'13px', textOverflow: 'ellipsis', overflow: 'hidden'};

  if (is_top_folder) {
    labelStyle.fontSize ='11px';
    labelStyle.textTransform = 'uppercase';
    labelStyle.fontWeight = 'bold';
  } else if (is_empty_folder) {
    // Wrap to show full message.
    labelStyle.whiteSpace = 'normal';
  } else {
    labelStyle.letterSpacing = 0;
    labelStyle.fontFamily = 'monospace';
  }

  return (
    <TreeItem2Provider itemId={itemId}>
      <TreeItem2Root {...getRootProps(other)}>
        <CustomTreeItemContent {...getContentProps()}>
          <TreeItem2IconContainer {...getIconContainerProps()}>
            <TreeItem2Icon status={status} />
          </TreeItem2IconContainer>
          
            {is_empty_folder ? (
              <Typography variant="caption" sx={labelStyle} {...getLabelProps()}>
                {empty_folder_label}
              </Typography>
            ) : (
              <Box display="flex" overflow="hidden" justifyContent="space-between" width="100%">
                <Box flexGrow={1} overflow="hidden" >
                  <div style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>
                    <span style={labelStyle} {...getLabelProps()} >
                      {labelInnerStyling(label)}
                    </span>
                  </div>
                  {/* <TreeItem2Label sx={labelSx} {...getLabelProps()} />*/}
                </Box>
                {to_clipboard && (
                  <Box width={20}>                                   
                    <CopyToClipboardButton text={to_clipboard} message="Copied!" />
                  </Box>
                )}
              </Box>              
            )}         
          
        </CustomTreeItemContent>
        {children && <TreeItem2GroupTransition {...getGroupTransitionProps()} />}        
      </TreeItem2Root>
    </TreeItem2Provider>
  );
});


interface MuiTreeViewProps {  
  items: TreeViewBaseItem[];
  workdir: string;
}
  
export default function MuiTreeView({ items, workdir }: MuiTreeViewProps) {
    return (    
      <RichTreeView
        aria-label="icon expansion"
        sx={{ position: 'relative' }}
        defaultExpandedItems={['3']}
        items={items}
        slots={{ item: (props: any) => <CustomTreeItem {...props} workdir={workdir} /> }}
      />    
    );
}