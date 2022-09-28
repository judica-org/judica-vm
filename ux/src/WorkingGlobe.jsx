import { appWindow } from '@tauri-apps/api/window';
import React from "react";
import countries_data from "./countries.json";
import earth from "./earth-dark.jpeg";
import Globe from "react-globe.gl";
import { Card, CardHeader, CardContent } from '@mui/material';
import { emit } from '@tauri-apps/api/event';
const { useState, useEffect } = React;

const stub_plant_data = [{
    id: 45637,
    lat: 46.818188,
    lng: 8.227512,
    plant_type: "Flare",
    text: "Flare Plant",
    owner: 12345566,
    watts: 345634,
    for_sale: false
}, {
    id: 13436,
    lat: 49.817492,
    lng: 15.472962,
    plant_type: "Hydro",
    text: "Hydroelectric Plant",
    owner: 12345566,
    watts: 67897,
    for_sale: false
}, {
    id: 95944,
    lat: 9.145,
    lng: 40.489673,
    plant_type: "Solar",
    text: "Solar Farm",
    owner: 12345566,
    watts: 23795,
    for_sale: true
}]

export default () => {
    const [power_plants, set_power_plants] = useState([]); // use empty list for now so it will render
    const [countries, setCountries] = useState([]);
    const [location, setLocation] = useState(null);
    const [selected_plant, setSelectedPlant] = useState(null);

    useEffect(() => {
        setCountries(countries_data);
        const unlisten_power_plants = appWindow.listen("power-plants", (ev) => {
            console.log(['game-board-event'], ev);
            set_power_plants(ev.payload)
        });

        return () => {
            (async () => {
                (await unlisten_power_plants)();
            })();
        }
    }, [power_plants, location]);

    return <div className='globe-container'>
        <Card>
            <CardHeader title={'World Energy Grid'}
                subheader={location ? `Selected Location: ${location.lat}, ${location.lng}` : 'Click to select location'}
            />
            <CardContent  >
                <div className='GlobeContent'>
                    <Globe
                        onHexPolygonClick={(_polygon, _ev, { lat, lng }) => {
                            setLocation({ lat, lng })
                            console.log(['globe-click'], { lat, lng });
                            emit('globe-click', [lat, lng]);
                        }}
                        globeImageUrl={earth}
                        width={600}
                        height={600}
                        labelsData={power_plants}
                        labelText={'text'}
                        labelSize={2}
                        labelColor={() => 'white'}
                        labelAltitude={0.1}
                        labelIncludeDot={true}
                        labelDotRadius={0.5}
                        labelDotOrientation={() => 'top'}
                        labelLabel={
                            (l) => `
                            <b>ID: ${l.id}</b> <br />
                            Owner: <i>${l.owner}</i> <br />
                            Watts: <i>${l.watts}</i> <br />
                            ${l.for_sale ? 'For Sale' : ''}
                            `
                        }
                        onLabelClick={(label, _ev, _data) => {
                            setSelectedPlant(label);
                            console.log(['label-click'], label);
                            emit('plant-selected', label);
                        }}
                        hexPolygonsData={countries.features}
                        hexPolygonResolution={3}
                        hexPolygonMargin={0.3}
                        hexPolygonColor={
                            () => `#${Math.round(Math.random() * Math.pow(2, 24)).toString(16).padStart(6, '0')}`
                        }
                        hexPolygonLabel={
                            ({ properties: d }) => `
        <b>${d.ADMIN} (${d.ISO_A2})</b> <br />
        Population: <i>${d.POP_EST}</i>
      `
                        }
                    />
                </div>
            </CardContent>
        </Card>
    </div>;
};