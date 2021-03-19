{{- define "comm.container" }}
- name: {{ .Chart.Name }}
  image: {{ .Values.global.image.prefix }}{{ .Chart.Name }}:{{ .Values.global.image.tag }}
  imagePullPolicy: {{ .Values.global.image.pullPolicy }}
  {{- with .Values.resources }}
  resources: 
      {{- toYaml . | nindent 4 }}
  {{- end }}
  env:
    - name: JJ_SERVICE_IP
      valueFrom:
        fieldRef:
          fieldPath: status.podIP
    - name: JJ_SERVICE_NAME
      valueFrom:
        fieldRef:
          fieldPath: metadata.name
    - name: JJ_SERVICE_MODULE
      value: {{ .Chart.Name }}
  envFrom:
    - configMapRef:
        name: {{ .Release.Name }}-cfg
    - secretRef:
        name: {{ .Release.Name }}-secret
  ports:
    - containerPort: 11100
      name: {{ .Chart.Name }}
{{- end }}


{{- define "gateway.deployment" }}
apiVersion: apps/v1
kind: Deployment
metadata: 
  name: {{ .Chart.Name }}
  namespace: {{ .Release.Namespace }}
spec:
  selector:
    matchLabels:
      name: {{ .Chart.Name }}
  replicas: {{ .Values.service.replicas }}
  template: 
    metadata:
      labels:
        name: {{ .Chart.Name }}
    spec:
      containers:
{{- include "comm.container" . | indent 8 }}
      imagePullSecrets:
        - name: {{ .Values.global.image.pullSecrets }}
{{- end }}

{{- define "gateway.service" }}
apiVersion: v1
kind: Service
metadata:
  name: {{ .Chart.Name }}
  namespace: {{ .Release.Namespace }}
spec:
  type: NodePort
  selector: 
    name: {{ .Chart.Name }}
  ports:
    - targetPort: 11100
      nodePort: {{ .Values.service.nodePort }}
      port: {{ .Values.service.port }}
      name: {{ .Chart.Name }}
{{- end }}

{{- define "comm.service" }}
apiVersion: v1
kind: Service
metadata:
  name: {{ .Chart.Name }}
  namespace: {{ .Release.Namespace }}
spec:
  clusterIP: None
  selector: 
    name: {{ .Chart.Name }}
  ports:
    - targetPort: 11100
      port: {{ .Values.service.port }}
      name: {{ .Chart.Name }}
{{- end }}

{{- define "comm.deployment" }}
apiVersion: apps/v1
kind: Deployment
metadata: 
  name: {{ .Chart.Name }}
  namespace: {{ .Release.Namespace }}
spec:
  selector:
    matchLabels:
      name: {{ .Chart.Name }}
  replicas: {{ .Values.service.replicas }}
  template: 
    metadata:
      labels:
        name: {{ .Chart.Name }}
    spec:
      containers:
{{- include "comm.container" . | indent 8 }}
      imagePullSecrets:
        - name: {{ .Values.global.image.pullSecrets }}
{{- end }}

{{- define "stateful.service" }}
apiVersion: v1
kind: Service
metadata:
  name: {{ .Chart.Name }}
  namespace: {{ .Release.Namespace }}
spec:
  clusterIP: None
  selector: 
    name: {{ .Chart.Name }}
  ports:
    - targetPort: 11100
      port: {{ .Values.service.port }}
      name: {{ .Chart.Name }}
{{- end }}

{{- define "stateful.deployment" }}
apiVersion: apps/v1
kind: StatefulSet
metadata: 
  name: {{ .Chart.Name }}
  namespace: {{ .Release.Namespace }}
spec:
  selector:
    matchLabels:
      name: {{ .Chart.Name }}
  serviceName: {{ .Chart.Name }}
  replicas: {{ .Values.service.replicas }}
  template: 
    metadata:
      labels:
        name: {{ .Chart.Name }}
    spec:
      containers:
{{- include "comm.container" . | indent 8 }}
      imagePullSecrets:
        - name: {{ .Values.global.image.pullSecrets }}
{{- end }}
