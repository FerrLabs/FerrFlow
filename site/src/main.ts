import '@angular/localize/init';
import { bootstrapApplication } from '@angular/platform-browser';
import { appConfig } from './app/app.config';
import { AppComponent } from './app/app.component';
import { initTelemetry } from '@ferrlabs/telemetry';

initTelemetry({ appName: 'ferrflow-site' });

try {
  await bootstrapApplication(AppComponent, appConfig);
} catch (err) {
  console.error(err);
}
