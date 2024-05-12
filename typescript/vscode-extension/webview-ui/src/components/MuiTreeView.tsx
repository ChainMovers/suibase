import * as React from 'react';
import { styled, SxProps, Theme } from '@mui/material/styles';

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
  TreeItem2Label,
  TreeItem2Root,
} from '@mui/x-tree-view/TreeItem2';
import { TreeItem2Icon } from '@mui/x-tree-view/TreeItem2Icon';
import { TreeItem2Provider } from '@mui/x-tree-view/TreeItem2Provider';
import { Typography } from '@mui/material';
import { TREE_ITEM_ID_RECENT_PACKAGES_EMPTY } from '../common/Consts';

const CustomTreeItemContent = styled(TreeItem2Content)(({ }) => ({
  padding: 0,
}));


interface CustomTreeItemProps
  extends Omit<UseTreeItem2Parameters, 'rootRef'>,
    Omit<React.HTMLAttributes<HTMLLIElement>, 'onFocus'> 
{
  workdir: string;
}

const CustomTreeItem = React.forwardRef(function CustomTreeItem(
  props: CustomTreeItemProps,
  ref: React.Ref<HTMLLIElement>,
) {
  const { workdir, id, itemId, label, disabled, children, ...other } = props;

  // set is_top_folder to true if first character of itemId is a numeric.
  // See Consts.ts for the meaning of the first char ( TREE_ITEM_x ).
  let is_top_folder = false;
  let is_empty_folder = false;
  let empty_folder_label = '(empty)';
  const first_char = itemId.charAt(0);
  if (first_char.length > 0) {
    if (first_char >= '0' && first_char <= '9') {
        is_top_folder = true;
    } else if (first_char === 'x') {
        is_empty_folder = true;
        if (itemId === TREE_ITEM_ID_RECENT_PACKAGES_EMPTY) {
          empty_folder_label = `To get started, do '${workdir} publish' in a terminal`;
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

  let labelSx: SxProps<Theme> = { p: 0, fontSize:'11px', whiteSpace: 'nowrap', overflow: 'hidden'};

  if (is_top_folder) {
    // Add some styles to labelSx.
    labelSx.textTransform = 'uppercase';
    labelSx.fontWeight = 'bold';
  } else if (is_empty_folder) {
    // Wrap to show full message.
    labelSx.whiteSpace = 'normal';
  }

  return (
    <TreeItem2Provider itemId={itemId}>
      <TreeItem2Root {...getRootProps(other)}>
        <CustomTreeItemContent {...getContentProps()}>
          <TreeItem2IconContainer {...getIconContainerProps()}>
            <TreeItem2Icon status={status} />
          </TreeItem2IconContainer>
          <Box>      
            {is_empty_folder ? (
              <Typography variant="caption" sx={labelSx} {...getLabelProps()}>
                {empty_folder_label}
              </Typography>
            ) : (
              <TreeItem2Label sx={labelSx} {...getLabelProps()} />
            )}
          </Box>
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
    <Box sx={{flexGrow: 1}}>
      <RichTreeView
        aria-label="icon expansion"
        sx={{ position: 'relative' }}
        defaultExpandedItems={['3']}
        items={items}
        slots={{ item: (props: any) => <CustomTreeItem {...props} workdir={workdir} /> }}
      />
      </Box>
    );
  /*return (
    <Box flex-direction="column" display="flex">
      <RichTreeView items={items} />
    </Box>
  );*/
}