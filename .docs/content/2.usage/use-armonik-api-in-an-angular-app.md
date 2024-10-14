---
description: This page will guide you through the process of using ArmoniK API in an Angular App.
---

# Use ArmoniK API in an Angular App

The purpose of this guide is to explain how to use ArmoniK API in an Angular App. You will be guided you through the process of creating a new Angular App and install ArmoniK API in it and use it.

At the end of the guide, you will have a working Angular App that uses ArmoniK API and you will be able to use it as a starting point for your own project or to contribute to the ArmoniK Admin GUI!

During this guide, we will follow the [community guidelines from ArmoniK](https://aneoconsulting.github.io/ArmoniK.Community/contribution-guides/angular)

::list
- Use Angular project structure without [Nx](https://nx.dev/)
- Use [standalone component](https://angular.io/guide/standalone-components)
- Use [inline template](https://angular.io/api/core/Component#template) and [inline style](https://angular.io/api/core/Component#styles)
- Use CSS
::

::alert{type="danger"}
This guide **is not** a tutorial on how to use Angular or RxJS. If you are not familiar with Angular or RxJS, we recommend you to follow the [official tutorial](https://angular.io/tutorial) first and [learn RxJS](https://www.learnrxjs.io/).
::

## Prerequisites

Before you start, make sure you have the following:

- [Node.js](https://nodejs.org/en/) installed on your machine.
- [ArmoniK] up and running on your machine. Follow the [installation guide](https://aneoconsulting.github.io/ArmoniK/installation) to install ArmoniK if you haven't done it yet.

::alert{type="info"}
Please make sure you have the latest LTS version of Node.js installed.
::

## Create a new Angular App

To create a new Angular App, you can use the [Angular CLI](https://angular.io/cli).

```bash
npm install -g @angular/cli
```

Then, you can create a new Angular App using the following command:

```bash
ng new --standalone --routing=false --inline-style --inline-template --skip-tests --skip-git --style=css --skip-install --package-manager=pnpm armonik-api-angular
```

Then, go to the newly created folder:

```bash
cd armonik-api-angular
```

And install the dependencies:

```bash
pnpm install
```

::alert{type="info"}
We recommend the use of [pnpm](https://pnpm.io/) to install the dependencies. It is faster and more efficient than `npm` or `yarn`.
::

## Clean the project

In the `src/app/app.component.ts` file, replace the content with the following:

```typescript [app.component.ts]
import { Component } from '@angular/core';

@Component({
  selector: 'app-root',
  template: `
  `,
  styles: [`
  `],
  standalone: true,
  imports: [],
  providers: [],
})
export class AppComponent {
}
```

## Install ArmoniK API

In order to be able to use ArmoniK API, you need to install the package `@aneoconsultingfr/armonik.api.angular` and some other dependencies.

```bash
pnpm install @ngx-grpc/common @ngx-grpc/core @ngx-grpc/grpc-web-client google-protobuf @types/google-protobuf @aneoconsultingfr/armonik.api.angular
```

`google-protobuf` is a dependency that will be used to serialize and deserialize the messages.

::alert{type="info"}
You can read more about `google-protobuf` in the [official documentation](https://www.npmjs.com/package/google-protobuf).
::

`@ngx-grpc/common` `@ngx-grpc/core` `@ngx-grpc/grpc-web-client` are dependencies that will allow us to use gRPC in our Angular App.

::alert{type="info"}
You can read more about `@ngx-grpc` in the [official documentation](https://github.com/ngx-grpc/ngx-grpc).
::

`@aneoconsultingfr/armonik.api.angular` is the package that contains the ArmoniK API (definition of the gRPC services and the generated code).

::alert{type="info"}
You can read more about `@aneoconsultingfr/armonik.api.angular` in the [package documentation](https://www.npmjs.com/package/@aneoconsultingfr/armonik.api.angular).
::

## Configure the Angular App

In order to be able to call some gRPC services, you need to configure the Angular App to use gRPC. In the config file, you will import the providers from the `@ngx-grpc/core` and `@ngx-grpc/grpc-web-client` packages.

In the `src/app/app.config.ts` file, you should add the following content:

```typescript [app.config.ts]
import { ApplicationConfig, importProvidersFrom } from '@angular/core';
import { GrpcCoreModule } from '@ngx-grpc/core';
import { GrpcWebClientModule } from '@ngx-grpc/grpc-web-client';

export const appConfig: ApplicationConfig = {
  providers: [
    importProvidersFrom(GrpcCoreModule.forRoot()),
    importProvidersFrom(GrpcWebClientModule.forRoot({ settings: { host: '' } }))
  ]
};
```

That's it! Now, you are ready to do some gRPC calls!

## Call the gRPC services

In this section, you will get partitions ArmoniK.

::alert{type="info"}
We chose ListPartitionsservice as it is a simple service and the data feedback is sure to happen. You may use another service if you so choose. If you do, we recommend to [run some samples](https://aneoconsulting.github.io/ArmoniK/installation/linux/verify-installation#samples) in ArmoniK to make sure that the service will return some data.
::

### Create the service

First, you need to create a folder to store our services.

```bash
mkdir src/app/services
```

Then, you can create a new file `src/app/services/partitions-grpc.service.ts` and create a service:

```typescript [partitions-grpc.service.ts]
import { Injectable } from '@angular/core';

@Injectable()
export class PartitionsGrpcService {
}
```

::alert{type="info"}
In Angular, it is a convention to add the `.service` suffix in the filename and to name the service with the suffix `Service`.
::

Then, you will inject the `ResultClient` in the service:

```typescript [partitions-grpc.service.ts]
import { Injectable, inject } from '@angular/core';
import { PartitionsClient } from '@aneoconsultingfr/armonik.api.angular';

@Injectable()
export class PartitionsGrpcService {
  readonly #client = inject(PartitionsClient);
}
```

::alert{type="info"}
You use the [`inject`](https://angular.io/api/core/inject) function to inject the `ResultsClient` in the service. You can also use the constructor to inject the client but it is not recommended because it will make the service harder to test. Using the `inject` method allow us to harmonize the way you inject the dependencies in services, components, etc.
::

Then, you can create a method to call the `listPartitions` service:

```typescript [partitions-grpc.service.ts]
import { Injectable, inject } from '@angular/core';
import { ListPartitionsRequest, ListPartitionsResponse, PartitionsClient } from '@aneoconsultingfr/armonik.api.angular';
import { Observable } from 'rxjs';

@Injectable()
export class PartitionsGrpcService {
  readonly #client = inject(PartitionsClient);

  list$(): Observable<ListPartitionsResponse> {
    const options = new ListPartitionsRequest();

    return this.#client.listPartitions(options);
  }
}
```

_Voilà!_ You've created our first gRPC service! Now, we can use it in our component.

### Create the component

::alert{type="info"}
In order to simplify this guide, will use the `AppComponent` to call the service. In a real application, you should create a dedicated component using the router.
::

For this guide, you will display the result in a list, having a loading indicator, a button to refresh the list and an error message if the call failed (printed in the console).

#### Display partitions

First, let's create the list using HTML:

```html [app.component.ts]
<ul>
  <li *ngFor="let partition of partitions; trackBy:trackByPartition">
    {{ partition.id }}
  </li>
</ul>
```

::alert{type="info"}
You must add this code in the `template` property of the `@Component` decorator.
::

Then, you must update the component with some properties and methods:

```typescript [app.component.ts]
import { PartitionRaw } from '@aneoconsultingfr/armonik.api.angular';

@Component({
  // ...
})
export class AppComponent {
  // ...
  partitions: PartitionRaw.AsObject[] = [];

  trackByPartition(_index_: number, partition: PartitionRaw.AsObject): string {
    return partition.id;
  }
}
```

Finally., you must import `ngFor` in the `imports` property of the `@Component` decorator:

```typescript [app.component.ts]
import { Component } from '@angular/core';
import { NgFor } from '@angular/common';

@Component({
  // ...
  imports: [
    NgFor
  ],
  // ...
})
```

#### Display the loading indicator

You will use the `*ngIf` directive to display the loading indicator:

```html [app.component.ts]
<div *ngIf="loading">
  Loading...
</div>
```

::alert{type="info"}
You must add this code in the `template` property of the `@Component` decorator.
::

Then, you must update the component with some properties and methods:

```typescript [app.component.ts]
@Component({
  // ...
})
export class AppComponent {
  // ...
  loading = true;
}
```

::alert{type="info"}
By default, the loading indicator will be displayed because data will be fetched on page initialization. You will hide it when the call is done.
::

Finally, you must import `ngIf` in the `imports` property of the `@Component` decorator:

```typescript [app.component.ts]
import { Component } from '@angular/core';
import { NgIf } from '@angular/common';

@Component({
  // ...
  imports: [
    NgIf
  ]
})
```

#### Display the refresh button

You will use the `button` element to display the refresh button:

```html [app.component.ts]
<button (click)="refresh()">Refresh</button>
```

::alert{type="info"}
You must add this code in the `template` property of the `@Component` decorator.
::

Then, you must update our component with some properties and methods:

```typescript [app.component.ts]
@Component({
  // ...
})
export class AppComponent {
  // ...
  refresh(): void {
    // You will use this method later, when you will call the gRPC service.
  }
}
```

### Use the service

Now that you've created our service and our component, you are ready to use them together.

First, you need to inject the service in the component:

```typescript [app.component.ts]
import { Component, inject } from '@angular/core';
import { PartitionsGrpcService } from './services/partitions-grpc.service';

@Component({
  providers: [
    PartitionsGrpcService
  ]
})
export class AppComponent {
  // ...
  #partitionsGrpcService = inject(PartitionsGrpcService);
}
```

::alert{type="warning"}
We must add the service in the `providers` property of the `@Component` decorator.
::

Then, we will first call data after view init:

```typescript [app.component.ts]
import { Component, AfterViewInit } from '@angular/core';
import { merge, startWith, switchMap } from 'rxjs';

@Component({
  // ...
})
export class AppComponent implements AfterViewInit {
  // ...
  ngAfterViewInit(): void { // You use the AfterViewInit lifecycle hook in order to be sure that the view is initialized.

    merge() // You use the merge operator to call the service when the component is initialized and when the user click on the refresh button (implemented later).
      .pipe(
        startWith({}), // You use the startWith operator to call the service when the component is initialized.
        switchMap(() => { // You use the switchMap operator to cancel the previous call when the user click on the refresh button.
          this.loading = true; // You display the loading indicator while the call is in progress.
          return this.#partitionsGrpcService.list$();
        }),
      )
      .subscribe(
        (response) => {
          this.loading = false; // You hide the loading indicator when the call is done.

          if (response.partitions) {
            this.partitions = response.partitions; // You update the partitions list.
          }
        }
      );
  }
}
```

:x: But **it did not work**! We have an error in the console:

```text
app.component.ts:45
  POST http://localhost:4200/armonik.api.grpc.v1.partitions.Partitions/ListPartitions 404 (Not Found)
```

In fact, you have to use a proxy in order to redirect the gRPC call to the gRPC server (ArmoniK in our case).

#### Create the proxy

First, you need to create a `proxy.conf.json` file in `src` folder of our project:

```json [proxy.conf.json]
{
  "/armonik.api.grpc.v1": {
    "target": "http://<ip:port>", // Replace <ip:port> by the IP and port of your ArmoniK server.
    "secure": false
  }
}
```

::alert{type="info"}
We recommend to add this file to your `.gitignore` file. In fact, this file is specific to your local environment. In order to provide a template for your team, you can create a `proxy.conf.json.example` file and add it to your repository.
::

Then, you must update our config file `angular.json` in order to use the proxy:

```json [angular.json]
{
  // ...
  "projects": {
    "armonik-api-angular": {
      // ...
      "architect": {
        "serve": {
          // ...
          "options": {
            // ...
            "proxyConfig": "src/proxy.conf.json"
          }
        }
      }
    }
  }
}
```

Now, you can restart our dev server:

```bash
pnpm run start
```

If you look at the console, you will see another error:

```json
{
  "statusCode": 3,
  "statusMessage": "Property PageSize failed validation.\nProperty Filter failed validation.\nProperty Filter failed validation.\nProperty Sort.Field failed validation.\nProperty Sort.Field failed validation.",
  "metadata": {
    "map": {}
  }
}
```

But that's a good error ! It means that you have successfully called the gRPC server.

#### Fix the error

For simplicity, you will update the service directly. In a real world scenario, you should pass params through the method.

```diff [partitions-grpc.service.ts]
  list$(/** You should pass params here in a real world app. */): Observable<ListPartitionsResponse> {
-   const options = new ListPartitionsRequest()
+   const options = new ListPartitionsRequest({
+     page: 0,
+     pageSize: 10,
+     sort: {
+       direction: SortDirection.SORT_DIRECTION_ASC,
+       field: {
+         partitionRawField: {
+           field: PartitionRawEnumField.PARTITION_RAW_ENUM_FIELD_ID
+         }
+       }
+     },
+     filters: {}
+   });

    return this.#client.listPartitions(options);
  }
```

And _voilà_! You have successfully called the gRPC server and displayed the result in our Angular app.

You must see the loading indicator disappear and the partitions list displayed (with only one partition named `default`).

#### Add the refresh button

Now, you want to be able to refresh data when the user clicks on the refresh button. In order to do so, you will use a new subject and emit a value when the user clicks on the button.

```typescript [app.component.ts]
import { Component, AfterViewInit } from '@angular/core';
import { merge, startWith, switchMap, Subject } from 'rxjs';

@Component({
  // ...
})
export class AppComponent implements AfterViewInit {
  // ...
  #refresh: Subject<void> = new Subject<void>();

  ngAfterViewInit(): void {
    merge(
      this.#refresh$, // You add the refresh$ subject to the merge function.
      // The usage of the merge function allow us be able to have multiple sources of data like a manual refresh, a timer, etc.
    )
      .pipe(
        startWith({}),
        switchMap(() => {
          this.loading = true;
          return this.#partitionsGrpcService.list$();
        }),
      )
      .subscribe(
        (response) => {
          this.loading = false;

          if (response.partitions) {
            this.partitions = response.partitions;
          }
        }
      );

  }

  refresh(): void {
    this.#refresh$.next();
  }
}
```

Now, you can click on the refresh button! You will see the loading indicator appear and disappear and a new network call will be made (in the network tab of your inspector).

## Next steps

Now, you can continue to explore the ArmoniK API and create your own GUI.

Here are some ideas:

- Use a router to navigate between the different pages. Each page could display a different resource (partitions, tasks, etc.).
- Add some styles to your app.
- Add a auto-refresh feature to automatically refresh data every X seconds.
- ...

## Conclusion

In this tutorial, you've seen how to create a simple Angular app which fetch data from the ArmoniK API. You have seen how to use the gRPC client generated by the ArmoniK API and how to use the RxJS operators to handle the data flow. We have also seen how to use a proxy in order to redirect the gRPC call to the gRPC server (ArmoniK in our case). Finally, you have seen how to use the merge operator to handle multiple sources of data. We hope that this tutorial will help you to create your own GUI for the ArmoniK API.

Feel free to open an issue if you have any question or if you want to suggest an improvement or a PR if you want to contribute to this tutorial.

You can find the source code of this tutorial on [GitHub](https://github.com/aneoconsulting/ArmoniK.Api/tree/main/examples/angular).
