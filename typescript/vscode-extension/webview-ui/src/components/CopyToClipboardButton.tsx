import { useState } from 'react'
import {
  Unstable_Popup as BasePopup
} from '@mui/base/Unstable_Popup';
import React from 'react';
import { Box, ClickAwayListener, useTheme} from '@mui/material';
import Fade from '@mui/material/Fade';

interface CopyToClipboardButtonProps {
    text: string;
    message?: string; // What is briefly displayed after copying.
}

const CopyToClipboardButton = ({text,message}: CopyToClipboardButtonProps) => {
    const theme = useTheme();
    const [anchor, setAnchor] = React.useState<null | HTMLElement>(null);
    const [open, setOpen] = useState(false)

    const handleClick = (event: React.MouseEvent<HTMLElement>) => {
      navigator.clipboard.writeText(text)
      setAnchor(anchor ? null : event.currentTarget);
      setOpen(true)
      setTimeout(() => {
        setOpen(false)
      }, 1000);
    };

    const handleClickAway = () => {
      setOpen(false)
    };
    
    // Default message to "{text} copied!" if message not specified.
    if (!message) {
      message = `Copied ${text}`
    }

    return (
        <>        
        <div className="icon" onClick={handleClick}><i className="codicon codicon-clippy"></i></div>
        <BasePopup
          id="placement-popper"
          open={open}
          anchor={anchor}
          placement='top-start'
          offset={4}
        >
          <ClickAwayListener onClickAway={handleClickAway}>
            <Fade in={open} timeout={1000}>
                <Box role="presentation" sx={{ padding: '2px', 
                    color: theme.palette.secondary.contrastText,
                    backgroundColor: theme.palette.secondary.main,
                    borderRadius: '2px' 
                   }}>
                {message}
                </Box>
            </Fade>
          </ClickAwayListener>      
        </BasePopup>
        </>
    );
}

export default CopyToClipboardButton