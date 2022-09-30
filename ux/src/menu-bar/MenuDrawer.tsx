import * as React from 'react';
import AppBar from '@mui/material/AppBar';
import Box from '@mui/material/Box';
import Divider from '@mui/material/Divider';
import Drawer from '@mui/material/Drawer';
import IconButton from '@mui/material/IconButton';
import SettingsIcon from '@mui/icons-material/Settings';
import Toolbar from '@mui/material/Toolbar';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import { AppHeader } from '../header/AppHeader';
import Close from '@mui/icons-material/Close';
import { List, ListItem, ListItemButton, ListItemIcon, ListItemText } from '@mui/material';
import { Bolt, Handyman, Sell, Send, ShoppingCart } from '@mui/icons-material';
import { SwitchToGameProps } from '../header/SwitchToGame';
import { KeySelectorProps } from '../header/KeySelector';

interface Props extends SwitchToGameProps, KeySelectorProps {
  db_name_loaded: [string, string | null] | null;
};

const settingsDrawerWidth = '100vw';


export default function DrawerAppBar({ db_name_loaded, which_game_loaded, available_sequencers, signing_key, available_keys }: Props) {
  const gameMoves = false;
  const [mobileOpen, setMobileOpen] = React.useState(false);
  const [moveMenuOpen, setMoveMenuOpen] = React.useState(false);

  const handleDrawerToggle = () => {
    setMobileOpen(!mobileOpen);
  };

  const toggleMoveDrawer =
    (open: boolean) =>
      (event: React.KeyboardEvent | React.MouseEvent) => {
        if (
          event.type === 'keydown' &&
          ((event as React.KeyboardEvent).key === 'Tab' ||
            (event as React.KeyboardEvent).key === 'Shift')
        ) {
          return;
        }
        setMoveMenuOpen(open);
      };

  const drawer = (
    <Box sx={{ textAlign: 'center' }}>
      <Typography variant="h6" sx={{ my: 2 }}>
        <IconButton
          color="inherit"
          aria-label="close drawer"
          edge="start"
          onClick={handleDrawerToggle}
          sx={{ ml: 2 }}
        >
          <Close />
        </IconButton>
        Settings
      </Typography>
      <Divider />
      <AppHeader {...{ available_sequencers, which_game_loaded, db_name_loaded, signing_key, available_keys }}></AppHeader>
    </Box>
  );

  const moveList = () => (
    <Box
      sx={{ width: 250 }}
      role="presentation"
      onKeyDown={toggleMoveDrawer(false)}
    >
      <List>
        <ListItem>
          <IconButton
            color="inherit"
            aria-label="close drawer"
            edge="start"
            onClick={toggleMoveDrawer(false)}
            sx={{ ml: 1 }}
          >
            <Close />
          </IconButton>
        </ListItem>
      </List>
      <Divider />
      <List>
        <ListItem>
          <ListItemButton>
            <ListItemIcon>
              <Send />
            </ListItemIcon>
            <ListItemText primary={'Send Tokens'} />
          </ListItemButton>
        </ListItem>
        {['Buy Materials or Hashboards', 'Sell Materials or Hashboards'].map((text, index) => (
          <ListItem>
            <ListItemButton>
              <ListItemIcon>
                {index === 0 ? <ShoppingCart /> : <Sell />}
              </ListItemIcon>
              <ListItemText primary={text} />
            </ListItemButton>
          </ListItem>
        ))}
      </List>
      <List>
        {['Mint a Power Plant', 'SuperMint a Power Plant'].map((text, index) => (
          <ListItem>
            <ListItemButton>
              <ListItemIcon>
                {index === 0 ? <Handyman /> : <Bolt />}
              </ListItemIcon>
              <ListItemText primary={text} />
            </ListItemButton>
          </ListItem>
        ))}
        <ListItem>
          <ListItemButton>
            <ListItemIcon>
              <SettingsIcon />
            </ListItemIcon>
            <ListItemText primary={'Manage your Plower Plants'} />
          </ListItemButton>
        </ListItem>
      </List>
    </Box>
  )

  const container = undefined;

  return (
    <Box sx={{ display: 'flex' }}>
      <AppBar component="nav">
        <Toolbar>
          <IconButton
            color="inherit"
            aria-label="open drawer"
            edge="start"
            onClick={handleDrawerToggle}
            sx={{ mr: 2 }}
          >
            <SettingsIcon />
          </IconButton>
          <Typography
            variant="h6"
            component="div"
            sx={{ flexGrow: 1, display: { xs: 'none', sm: 'block' } }}
          >
            MINE WITH FRIENDS!
          </Typography>
          {gameMoves && <Box sx={{ display: { xs: 'none', sm: 'block' } }}>
            <Button key={"moves"} sx={{ color: '#fff' }} onClick={toggleMoveDrawer(true)}>
              {'GAME MOVES'}
            </Button>
            <Drawer anchor='right' open={moveMenuOpen} onClose={toggleMoveDrawer(false)}>
              {moveList()}
            </Drawer>
          </Box>}
        </Toolbar>
      </AppBar>
      <Box component="nav">
        <Drawer
          anchor='top'
          container={container}
          variant="temporary"
          open={mobileOpen}
          onClose={handleDrawerToggle}
          ModalProps={{
            keepMounted: true, // Better open performance on mobile.
          }}
          sx={{
            '& .MuiDrawer-paper': { boxSizing: 'border-box', width: settingsDrawerWidth },
          }}
        >
          {drawer}
        </Drawer>
      </Box>
      <Box component="main" sx={{ p: 0 }}>
        <Toolbar />
      </Box>
    </Box>
  );
}