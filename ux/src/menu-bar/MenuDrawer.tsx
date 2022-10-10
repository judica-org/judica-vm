import * as React from 'react';
import AppBar from '@mui/material/AppBar';
import Box from '@mui/material/Box';
import Divider from '@mui/material/Divider';
import Drawer from '@mui/material/Drawer';
import IconButton from '@mui/material/IconButton';
import SettingsIcon from '@mui/icons-material/Settings';
import Toolbar from '@mui/material/Toolbar';
import Typography from '@mui/material/Typography';
import { AppHeader } from '../header/AppHeader';
import Close from '@mui/icons-material/Close';
import { SwitchToGameProps } from '../header/SwitchToGame';
import { KeySelectorProps } from '../header/KeySelector';
import { NewGameProps } from '../header/NewGame';
import { SwitchToHostProps } from '../header/SwitchToHost';
import { EntityID } from '../Types/GameMove';

interface Props extends SwitchToGameProps, KeySelectorProps, NewGameProps, SwitchToHostProps {
  db_name_loaded: [string, string | null] | null;
  readonly user_id: EntityID | null;
  readonly elapsed_time: number|null;
};

const settingsDrawerWidth = '100vw';


export default function DrawerAppBar({ db_name_loaded,
  which_game_loaded,
  available_sequencers,
  signing_key,
  available_keys,
  join_code,
  join_password,
  game_host_service,
  user_id,
  elapsed_time
}:
  Props) {
  const [player_id, set_player_id] = React.useState<EntityID | null>(null)
  const [mobileOpen, setMobileOpen] = React.useState(false);

  const handleDrawerToggle = () => {
    setMobileOpen(!mobileOpen);
  };

  React.useEffect(() => {
    set_player_id(user_id);
  }, [user_id])

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
      <AppHeader {...{
        available_sequencers, which_game_loaded,
        db_name_loaded, signing_key,
        available_keys, join_code, join_password,
        game_host_service
      }}></AppHeader>
    </Box>
  );

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
          {player_id && <Box sx={{ display: { xs: 'none', sm: 'block' } }}>
            <Typography variant="h6">{`Playing as ${player_id}`}</Typography>
          </Box>}
          <Typography
            variant="h6"
            component="div"
            sx={{ flexGrow: 1, display: { xs: 'none', sm: 'block' } }}
          >
            MASTER MINE!
          </Typography>
          {elapsed_time && <Box sx={{ display: { xs: 'none', sm: 'block' } }}>
            <Typography variant="h6">{`Est Time Remaining: ${new Date(3600000-elapsed_time).toISOString().slice(11, 19)}`}</Typography>
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