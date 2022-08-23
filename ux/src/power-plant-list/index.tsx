import './styles.css';
import { Card, CardContent, CardHeader } from '@mui/material';
// import MapCanvas from '../map-canvas';
import { useEffect, useRef, useState } from 'react';


export type PowerPlant = {
  id: number,
  plant_type: string //how does PlantType enum show up
  watts: number,
  coordinates: number[],
  has_miners: boolean,
  for_sale: boolean,
}

export function PowerPlants({ power_plants }: { power_plants: PowerPlant[] }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [context, setContext] = useState<CanvasRenderingContext2D | null>(null);
  useEffect(() => {

    if (canvasRef.current) {
      const renderCtx = canvasRef.current.getContext('2d');

      if (renderCtx) {
        setContext(renderCtx);
      }
    }
    if (context) {
      const factoryImage = new Image(10, 10);
      factoryImage.src = 'https://upload.wikimedia.org/wikipedia/commons/b/bb/Icon_NuclearPowerPlant-blue.svg';
      context.drawImage(factoryImage, 150, 200);
    }

  }, [context]);

  return (
    <div>
      <div className='power-plant-container'>
        <Card>
          <CardHeader title={'Map'}
            subheader={'Current Energy Grid'}
          />
          <CardContent className={'content'} style={{
            position: 'relative'
          }}>
            {/* <MapCanvas width={400} height={400}/> */}
            <canvas ref={canvasRef} width={400} height={400} style={{ zIndex: 3, backgroundColor: 'transparent' }} id={'map-canvas'}></canvas>
            <img src='https://upload.wikimedia.org/wikipedia/commons/c/cb/Gondwana_420_Ma.png' alt='Fama Clamosa, CC BY-SA 4.0 <https://creativecommons.org/licenses/by-sa/4.0>, via Wikimedia Commons' style={{
              height: '400px',
              width: '400px',
              position: 'absolute',
              top: '4%',
              right: '4%',
              opacity: 1,
            }
            }></img>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}